pub mod cli;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDateTime, Utc, serde::{ts_milliseconds}};

#[derive(Deserialize, Serialize, Debug)]
pub struct SubscribeInfo<'a> {
    #[serde(rename = "type")]
    pub message_type: &'a str,
    #[serde(rename = "symbol")]
    pub stock_symbol: &'a str,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TickerInfo {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: f64,
    #[serde(rename = "v")]
    volume: f64,
    #[serde(with = "ts_milliseconds", rename = "t")]
    time: DateTime<Utc>,
    #[serde(rename = "c")]
    conditions: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Response<'a> {
    #[serde(rename = "type")]
    transaction_type: &'a str,
    #[serde(rename = "data")]
    transaction_data: Vec<TickerInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsMessage {
    Error {
        #[serde(rename = "msg")]
        message: String,
    },
    Ping,
    Response,
}


impl TickerInfo {
    fn default() -> Self {
        TickerInfo {
            symbol: "".parse().unwrap(),
            price: 0.0,
            volume: 0.0,
            time: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(61, 0), Utc),
            conditions: Some(vec![]),
        }
    }

    fn new(symbol: &str, price: f64, volume: f64, date_time: &DateTime<Utc>, conditions: &[String]) -> Self {
        TickerInfo {
            symbol: symbol.parse().unwrap(),
            price,
            volume,
            time: *date_time,
            conditions: Some(conditions.to_vec()),
        }
    }
}

impl <'a> Response<'a> {
    pub fn default() -> Self{
        Response{
            transaction_type: "trade",
            transaction_data: vec![]
        }
    }
}

impl <'a> SubscribeInfo<'a> {
    pub fn new(symbol: &'a str) -> Self {
        SubscribeInfo {
            message_type: "subscribe",
            stock_symbol: symbol
        }
    }
}
