//! Candlestick primitives
//! # candlestick
//!
//! This contains the necessary structure and functions to manage
//! all the needs the program has with regard to candlestick
//! information for a stock
use std::fs::File;
use chrono::{DateTime, SubsecRound, Utc};
use serde::{Deserialize, Serialize};
use crate::{RollingData};

/// `Candlestick` is a struct containing the necessary information
/// to represent a stock candlestick graph entry.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Candlestick{
    /// stock_symbol: represents the stock symbol
    #[serde(rename = "Symbol")]
    pub stock_symbol: String,
    #[serde(rename = "MinuteOfDay")]
    /// minute_of_hour: represents the minute for which
    /// the candlestick is being calculated
    pub minute_of_hour: DateTime<Utc>,
    #[serde(rename = "OpenPrice")]
    /// open_price: represents the first entry available
    /// for the given symbol at the given minute of hour
    pub open_price: f64,
    #[serde(rename = "ClosePrice")]
    /// close_price: represents the last entry available
    /// for the given symbol at the given minute of hour
    pub close_price: f64,
    #[serde(rename = "HighestPrice")]
    /// highest_price: represents the maximum price among
    /// the available entries
    pub highest_price: f64,
    #[serde(rename = "LowestPrice")]
    /// lowest_price: represents the minimum price among
    /// the avaiable entries
    pub lowest_price: f64,
    #[serde(rename = "Transactions")]
    /// total_transactions: represents the number of
    /// transactions made for the given stock at the
    /// given minute of hour
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

    /// `write_to_file`: serializes the struct instance and writes it the given file
    pub fn write_to_file(&self, file: &File){
        let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(file);
        writer.serialize(self).unwrap();
        writer.flush().unwrap();
    }
}

/// `calculate_candlestick` given a reference to a slice of RollingData,
/// if the slice is not empty, it calculates the candlestick by assigning the opening price
/// to the first element of the slice, the closing price to the last, and by comparing the
/// elements, it calculates the min and max values. If the slice is empty, None is returned.
///
/// # Arguments
///
/// `data` - a slice of RollingData for which the mean data calculation should be made
///
/// # Example
/// ```
/// use finnhub_ws::candlestick::calculate_candlestick;
/// use finnhub_ws::RollingData;
/// let items: Vec<RollingData> = Vec::new();
/// let rolling = calculate_candlestick(&items);
/// assert_eq!(rolling, None)
/// ```
///
/// ```
/// use finnhub_ws::RollingData;
/// use chrono::{SubsecRound, TimeZone, Utc};
/// use finnhub_ws::candlestick::{calculate_candlestick, Candlestick};
/// let mut items: Vec<RollingData> = Vec::new();
/// let r1 = RollingData{
///     price: 172.5,
///     symbol: "APPL".parse().unwrap(),
///     timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376),
///     write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794)
/// };
/// let r2 = RollingData{
///     price: 173.5,
///     symbol: "APPL".parse().unwrap(),
///     timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 197),
///     write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 798)
/// };
/// items.push(r1);
/// items.push(r2);
/// let rolling = calculate_candlestick(&items);
/// assert_eq!(rolling, Some(Candlestick{
///     stock_symbol: "APPL".parse().unwrap(),
///     total_transactions: 2,
///     close_price: 173.5,
///     open_price: 172.5,
///     highest_price: 173.5,
///     lowest_price: 172.5,
///     minute_of_hour: Utc::now().round_subsecs(0)
///  }));
/// ```
pub fn calculate_candlestick(data: &[RollingData]) -> Option<Candlestick> {
    if !data.is_empty(){
        let max_price = data.iter().map(|x| { x.price }).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        let min_price = data.iter().map(|x| { x.price }).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        return Some(Candlestick::new(data[0].price, data[data.len()-1].price, max_price, min_price, data.len() as u64, Utc::now().round_subsecs(0), data[0].symbol.parse().unwrap()));
    }
    None
}

