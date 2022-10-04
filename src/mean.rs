//! Mean Data primitives
//! # mean
//!
//! This contains the necessary structure and functions to manage
//! all the needs the program has with regard to 15 minute mean data
//! information for a stock
use std::fs::File;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::RollingData;

/// `MeanData` is a struct containing the necessary information
/// to represent the average price of a stock for a 15-minute
/// time period
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MeanData {
    /// symbol: represents the stock symbol
    pub symbol: String,
    /// start_date: represents the earliest datetime
    /// a transaction was made
    pub start_time: DateTime<Utc>,
    /// end_date: represents the latest datetime
    /// a transaction was made
    pub end_time: DateTime<Utc>,
    /// mean_price: represents the average stock price
    /// of that 15-minute period for the given stock
    pub mean_price: f64,
    /// transactions: represents the total number of
    /// transactions made in the 15-minute period
    /// for the given stock
    pub transactions: u64,
}

impl MeanData {
    fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>, mean_price: f64, transactions: u64, symbol: String) -> Self {
        MeanData {
            symbol,
            transactions,
            mean_price,
            start_time,
            end_time,
        }
    }
    /// `write_to_file`: serializes the struct instance and writes it the given file
    pub fn write_to_file(&self, file: &File) {
        let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(file);
        writer.serialize(self).unwrap();
        writer.flush().unwrap();
    }
}

/// `calculate_mean_data` given a reference to a slice of RollingData,
/// if the slice is not empty, it calculates the mean_data by assigning the min date
/// to the first element of the slice, the max data to the last, and calculates
/// the average price among the data. If the slice is empty, None is returned.
///
/// # Arguments
///
/// `data` - a slice of RollingData for which the mean data calculation should be made
///
/// # Example
/// ```
/// use finnhub_ws::mean::calculate_mean_data;
/// use finnhub_ws::RollingData;
/// let items: Vec<RollingData> = Vec::new();
/// let rolling = calculate_mean_data(&items);
/// assert_eq!(rolling, None)
/// ```
///
/// ```
/// use finnhub_ws::mean::{calculate_mean_data, MeanData};
/// use finnhub_ws::RollingData;
/// use chrono::{TimeZone, Utc};
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
/// let rolling = calculate_mean_data(&items);
/// assert_eq!(rolling, Some(MeanData{
///     symbol: "APPL".parse().unwrap(),
///     transactions: 2,
///     start_time: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794),
///     end_time: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 798),
///     mean_price: 173.0,
/// }));
/// ```
pub fn calculate_mean_data(data: &[RollingData]) -> Option<MeanData> {
    if !data.is_empty() {
        let max_date = data.iter().map(|x| { x.write_timestamp }).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let min_date = data.iter().map(|x| { x.write_timestamp }).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let mean_price: f64 = data.iter().fold(0.0, |mean_price, i| mean_price + i.price) / (data.len() as f64);
        return Some(MeanData::new(min_date, max_date, mean_price, data.len() as u64, data[0].symbol.parse().unwrap()));
    }
    None
}
