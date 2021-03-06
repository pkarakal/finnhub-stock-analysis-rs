pub mod cli;
pub mod stock_handle;
pub mod utils;
pub mod candlestick;
pub mod mean;
use std::{fs::{File, OpenOptions, create_dir_all}, path::{PathBuf}, io};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDateTime, Utc, serde::{ts_milliseconds}};
use csv::StringRecord;


trait CSVAble {
    fn vectorize(&self) -> Vec<String>;
    fn get_headers(&self) -> Vec<String>;
}

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
    pub symbol: String,
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
    pub transaction_type: &'a str,
    #[serde(rename = "data")]
    pub transaction_data: Vec<TickerInfo>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Ping<'a> {
    #[serde(rename = "type")]
    pub action_type: &'a str
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WsError<'a> {
    #[serde(rename = "msg")]
    pub message: &'a str
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "lowercase")]
pub enum WsMessage<'a> {
    #[serde(borrow = "'a")]
    Response(Response<'a>),
    #[serde(borrow = "'a")]
    Error(WsError<'a>),
    #[serde(borrow = "'a")]
    Ping(Ping<'a>)
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RollingData {
    pub symbol: String,
    pub price: f64,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub write_timestamp: DateTime<Utc>
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

    pub fn write_to_disk(&self, file: &File) {
        let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(file);
        writer.serialize(self.vectorize()).unwrap();
        writer.flush().unwrap();
    }

    pub fn check_file_empty(&self, file: &File) -> bool {
        // has headers has been set to false as it will skip the first record
        let mut reader = csv::ReaderBuilder::new().has_headers(false).from_reader(file);
        let mut rec = StringRecord::new();
        !reader.read_record(&mut rec).unwrap()
    }

    fn check_file_exists(&self, path: &PathBuf) -> File {
        create_dir_all(path.parent().unwrap()).unwrap();
        match OpenOptions::new()
            .write(true)
            .append(true)
            .read(true)
            .open(path) {
            Ok(f) => f,
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    let f = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .read(true)
                        .create(true)
                        .open(path)
                        .unwrap();
                    self.write_headers(&f);
                    f
                }
                _ => panic!("Problem opening the file")
            }
        }
    }

    pub fn write_headers(&self, f: &File) {
        let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(f);
        writer.serialize(self.get_headers()).unwrap();
        writer.flush().unwrap();
    }
}

impl CSVAble for TickerInfo {
    fn vectorize(&self) -> Vec<String> {
        return vec![self.symbol.clone(),
                    self.price.to_string(),
                    self.time.timestamp_millis().to_string(),
                    Utc::now().timestamp_millis().to_string(),
        ];
    }
    fn get_headers(&self) -> Vec<String> {
        return vec!["Symbol".to_string(),
                    "Price".to_string(),
                    "Timestamp".to_string(),
                    "WriteTimestamp".to_string()];
    }
}

impl<'a> Response<'a> {
    pub fn default() -> Self {
        Response {
            transaction_type: "trade",
            transaction_data: vec![],
        }
    }
}

impl<'a> SubscribeInfo<'a> {
    pub fn new(symbol: &'a str) -> Self {
        SubscribeInfo {
            message_type: "subscribe",
            stock_symbol: symbol,
        }
    }
}
