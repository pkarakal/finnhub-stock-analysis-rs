use std::borrow::Borrow;
use std::io::Split;
use futures_util::{future::{FutureExt}, pin_mut, SinkExt, StreamExt, select};
use futures_util::stream::SplitSink;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use url::form_urlencoded::byte_serialize;
use finnhub_ws::{Response, SubscribeInfo};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[tokio::main]
async fn main() -> Result<()> {
    let connect_addr = "wss://ws.finnhub.io?token=capm2q2ad3i1rqbdbqk0";

    let url = url::Url::parse(connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    let (mut write, read) = ws_stream.split();

    read_stdin(write).await;


    let read_future = read.for_each(|message| async {
        let x = &*message.unwrap().into_data();
        let data = serde_json::from_slice::<Response>(x).unwrap();
        println!("{:?}", data);
        let encoded: Vec<u8> = bincode::serialize(&data).unwrap();
        tokio::io::stdout().write_all(&*encoded).await.unwrap();
    });

    read_future.await;
    Ok(())
}

async fn read_stdin(mut tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>){
    let items = ["AAPL", "AMZN", "MSFT", "BINANCE:BTCUSDT", "GTLB"].iter().map(|item| {
        SubscribeInfo::new(item)
    }).map(|x1| {
        match serde_json::to_string(&x1) {
            Ok(res) => res,
            Err(e) => "".parse().unwrap()
        }
    }).collect::<Vec<String>>();
    for item in items {
        tx.send(Message::Text(item)).await.unwrap();
    }
}