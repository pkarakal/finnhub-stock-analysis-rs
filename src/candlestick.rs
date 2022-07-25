use std::fs::File;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{RollingData};

#[derive(Serialize, Deserialize, Debug)]
pub struct Candlestick{
    #[serde(rename = "Symbol")]
    pub stock_symbol: String,
    #[serde(rename = "MinuteOfDay")]
    pub minute_of_hour: DateTime<Utc>,
    #[serde(rename = "OpenPrice")]
    pub open_price: f64,
    #[serde(rename = "ClosePrice")]
    pub close_price: f64,
    #[serde(rename = "HighestPrice")]
    pub highest_price: f64,
    #[serde(rename = "LowestPrice")]
    pub lowest_price: f64,
    #[serde(rename = "Transactions")]
    pub total_transactions: u64
}

impl Candlestick{
    pub fn default() -> Self {
        Candlestick {
            open_price: 0.0,
            close_price: 0.0,
            highest_price: 0.0,
            lowest_price: 0.0,
            total_transactions: 0,
            stock_symbol: "".parse().unwrap(),
            minute_of_hour: Utc::now(),
        }
    }

    fn new(open: f64, close: f64, high: f64, low: f64, count: u64, minute: DateTime<Utc>, symbol: String) -> Self {
        Candlestick {
            open_price: open,
            close_price: close,
            highest_price: high,
            lowest_price: low,
            total_transactions: count,
            stock_symbol: symbol,
            minute_of_hour: minute,
        }
    }
    pub fn write_to_file(&self, file: &File){
        let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(file);
        writer.serialize(self).unwrap();
        writer.flush().unwrap();
    }
}

pub fn calculate_candlestick(data: &[RollingData]) -> Option<Candlestick> {
    if !data.is_empty(){
        let max_price = data.iter().map(|x| { x.price }).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        let min_price = data.iter().map(|x| { x.price }).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        return Some(Candlestick::new(data[0].price, data[data.len()-1].price, max_price, min_price, data.len() as u64, Utc::now(), data[0].symbol.parse().unwrap()));
    }
    None
}

