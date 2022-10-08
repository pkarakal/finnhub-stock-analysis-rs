use std::process::exit;
use std::sync::Arc;
use chrono::{DurationRound};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use tokio::{net::TcpStream, time::{self, Duration}};
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use finnhub_ws::{
    cli::cmd::CLIOptions, Response, SubscribeInfo, WsMessage,
    candlestick::calculate_candlestick,
    stock_handle::{initialize_mapper, StockHandle},
    mean::calculate_mean_data,
    utils::{create_dirs, find_items},
    RollingData
};
use clap::Parser;
use rayon::prelude::*;
use crossbeam_channel::{Sender};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = CLIOptions::parse();

    let connect_addr = format!("wss://ws.finnhub.io?token={}", opts.token);

    let url = url::Url::parse(&connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    let dirs = vec!["data/rolling", "data/candlestick", "data/mean"];
    dirs.iter().for_each(|x| {
        if !create_dirs(x) {
            eprintln!("Couldn't create directories");
            exit(1);
        }
    });

    let mapper = initialize_mapper(&opts.stocks);

    let mapper_a = Arc::clone(&mapper);
    let mapper_b = Arc::clone(&mapper);
    let mapper_c = Arc::clone(&mapper);
    subscribe_to_stocks(&mut write, &opts.stocks).await;
    let futures_vec = vec![
        tokio::spawn(async move {
            read_from_stream(&mut read, &mut write, &mapper_c).await;
        }),
        tokio::spawn(async move {
        let candlestick_txs: Vec<Sender<i64>> = mapper_a.iter().map(|x| {
            let (tx, _) = x.stock_channel.clone();
            tx
        }).collect();
        let mean_txs: Vec<Sender<i64>> = mapper_a.iter().map(|x| {
            let (tx, _) = x.rolling_mean_channel.clone();
            tx
        }).collect();
        tick(&candlestick_txs, &mean_txs).await;
    }), tokio::spawn(async move {
        let cs_pool = rayon::ThreadPoolBuilder::new().num_threads(2 * mapper_b.len()).build().unwrap();
        cs_pool.install(|| {
            mapper_b.par_iter().for_each(|x| {
                rayon::join(|| wait_for_candlestick(x), || wait_for_mean(x));
            });
        });
    })];

    futures::future::join_all(futures_vec).await;
    Ok(())
}

/// `tick` is being used to send a signal to threads waiting to calculate
/// the candlestick and 15-minute mean data
///
/// # Arguments
/// candlestick_txs : a reference to a vector of Sender which represents the threads calculating the
///      candlestick
/// mean_txs: a reference to a vector of Sender which represents the threads calculating the 15-minute
///      mean data
async fn tick(candlestick_txs: &[Sender<i64>], mean_txs: &[Sender<i64>]) {
    let mut interval = time::interval(Duration::from_secs(60));
    interval.tick().await;
    loop {
        interval.tick().await;
        for (_, (cs_tx, me_tx)) in candlestick_txs.iter().zip(mean_txs.iter()).enumerate() {
            let timestamp = chrono::Local::now().duration_trunc(chrono::Duration::minutes(1)).unwrap().timestamp();
            cs_tx.send(timestamp).unwrap();
            me_tx.send(timestamp).unwrap();
        }
    }
}

/// `read_from_stream` reads data from the websocket and converts a byte array to `WsMessage` enum instance
async fn read_from_stream(read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, write: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, mapper: &Vec<StockHandle>) {
    while let Some(message) = read.next().await {
        match message {
            Ok(d) => {
                let x = &*d.into_data();
                let data = serde_json::from_slice::<WsMessage>(x).unwrap();
                match data {
                    WsMessage::Response(resp) => { parse_message(&resp, mapper) }
                    WsMessage::Ping(ping) => {
                        println!("{:?}", ping);
                        write.send(Message::Pong("".into())).await.unwrap();
                        println!("Pong sent");
                    },
                    WsMessage::Error(err) => println!("{:?}", err.message)
                }
            }
            Err(ref e) => {
                println!("{:?}", e);
            }
        }
    }
}

/// `wait_for_candlestick` blocks until data is retrieved from the channel.
/// Then, it reads rolling data file and filters entries of the last minute,
/// calculates the candlestick and writes it back to a file used for
/// candlestick information
fn wait_for_candlestick(handle: &StockHandle) {
    let (_, rx) = handle.rolling_mean_channel.clone();
    let mut items: Vec<RollingData> = Vec::with_capacity(1000);
    loop {
        // this blocks the thread
        let timestamp = rx.recv().unwrap();
        let mut rf = handle.rolling_file.lock().unwrap();
        find_items(&mut rf, timestamp, 1, &mut items);
        // rf would get dropped at the end of the iteration,
        // but there is no need to keep the lock much longer than this point
        drop(rf);
        let cf = handle.candlestick_file.lock().unwrap();
        match calculate_candlestick(&items) {
            Some(cs) => {
                cs.write_to_file(&cf);
            }
            None => {
            }
        };
        // same as for rf, just a good practice
        drop(cf);
        items.clear();
        items.shrink_to(1000);
    }
}

/// `wait_for_mean` blocks until data is retrieved from the channel.
/// Then, it reads rolling data file and filters entries of the 15 minutes,
/// calculates the mean data and writes it back to a file used for
/// mean information
fn wait_for_mean(handle: &StockHandle) {
    let (_, rx) = handle.stock_channel.clone();
    let mut items: Vec<RollingData> = Vec::with_capacity(1000);
    loop {
        let timestamp = rx.recv().unwrap();
        let mut rf = handle.rolling_file.lock().unwrap();
        find_items(&mut rf, timestamp, 15, &mut items);
        // rf would get dropped at the end of the iteration,
        // but there is no need to keep the lock much longer than this point
        drop(rf);
        let mf = handle.mean_file.lock().unwrap();
        match calculate_mean_data(&items) {
            Some(md) => {
                md.write_to_file(&mf);
            }
            None => {
            }
        };
        // same as for rf, just a good practice
        drop(mf);
        items.clear();
        items.shrink_to(1000);

    }
}

/// `subscribe_to_stocks`: Given a channel and an array of strings containing the stock names,
/// it sends a websocket message to finnhub to subscribe to that stock
async fn subscribe_to_stocks(tx: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, stocks: &[String]) {
    let items = stocks.iter().map(|item| {
        SubscribeInfo::new(item)
    }).map(|x1| {
        match serde_json::to_string(&x1) {
            Ok(res) => res,
            Err(_e) => "".parse().unwrap()
        }
    }).collect::<Vec<String>>();
    for item in items {
        tx.send(Message::Text(item)).await.unwrap();
    }
}

/// `parse_message` given a response and a reference to a `StockHandle`,
/// for each transaction in the response, it writes them to the rolling
/// file keeping the file lock during the write operation. On first invocation
/// it checks to see, if the rolling file exists, otherwise it creates it.
fn parse_message(resp: &Response, mapper: &Vec<StockHandle>) {
    resp.transaction_data.par_iter().for_each(|x| {
        match mapper.iter().find(|s| s.stock_symbol == x.symbol) {
            Some(handle) => {
                handle.once_flag.call_once(|| {
                    let rf = handle.rolling_file.lock().unwrap();
                    if x.check_file_empty(&rf) {
                        x.write_headers(&rf)
                    }
                });
                x.write_to_disk(&handle.rolling_file.lock().unwrap())
            }
            None => {}
        }
    });
}
