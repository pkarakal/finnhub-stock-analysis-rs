use std::fs::File;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::RollingData;


#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MeanData {
    pub symbol: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub mean_price: f64,
    pub transactions: u64
}

impl MeanData {
    fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>, mean_price: f64, transactions: u64, symbol: String) -> Self {
        MeanData {
            symbol,
            transactions,
            mean_price,
            start_time,
            end_time
        }
    }
    pub fn write_to_file(&self, file: &File){
        let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(file);
        writer.serialize(self).unwrap();
        writer.flush().unwrap();
    }
}

pub fn calculate_mean_data(data: &[RollingData]) -> Option<MeanData> {
    if !data.is_empty(){
        let max_date = data.iter().map(|x| { x.write_timestamp }).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        let min_date = data.iter().map(|x| { x.write_timestamp }).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap();
        let mean_price: f64 = data.iter().fold(0.0, |mean_price, i| mean_price + i.price) /( data.len() as f64);
        return Some(MeanData::new(min_date, max_date, mean_price, data.len() as u64, data[0].symbol.parse().unwrap()));
    }
    None
}
