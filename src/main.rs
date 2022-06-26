use std::fmt::format;
use std::path::{Path, PathBuf};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::io::{AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use finnhub_ws::{cli::cmd::CLIOptions, Response, SubscribeInfo};
use clap::Parser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = CLIOptions::parse();

    let connect_addr = format!("wss://ws.finnhub.io?token={}", opts.token);

    let url = url::Url::parse(&connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    let (write, read) = ws_stream.split();

    subscribe_to_stocks(write, &opts.stocks).await;


    let read_future = read.for_each(|message| async {
        let x = &*message.unwrap().into_data();
        let data = serde_json::from_slice::<Response>(x).unwrap();
        parse_message(&data)
    });

    read_future.await;
    Ok(())
}

async fn subscribe_to_stocks(mut tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, stocks: &[String]) {
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

fn parse_message(resp: &Response) {
    resp.transaction_data.iter().for_each(|x|  {
        x.write_to_disk(&PathBuf::from(format!("data/{}.csv", x.symbol)));
    });
}
