//! # finnhub_ws
//! This project is implemented as a part of a homework exercise for [077] - Real Time
//! Embedded Systems course of ECE Department, AUTh. This fetches stock information changes
//! as they happen in real time, using websockets, runs some analysis and persists the
//! information as it becomes available. It should consume the least cpu it can as it
//! should be able to be deployed in a microcontroller.
//!
//! # High level implementation
//!
//! This uses Tokio and Tungstenite to connect to finnhub.io using their websocket APIs, subscribes
//! to stocks and whenever a new transaction is made and obtained by the listening channel,
//! it gets written to the corresponding rolling file. Each minute and for each stock, a message is
//! sent to other channels to calculate the candlestick for the last minute alongside mean price of
//! each stock for the last fifteen minutes. The results are then written back to a file, a separate
//! one for each stock.
pub mod cli;
pub mod stock_handle;
pub mod utils;
pub mod candlestick;
pub mod mean;
use std::{fs::{File, OpenOptions, create_dir_all}, path::{PathBuf}, io};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, serde::{ts_milliseconds}};
use csv::StringRecord;

/// ```
/// trait CSVAble {
///     fn vectorize(&self) -> Vec<String>;
///     fn get_headers(&self) -> Vec<String>;
/// }
/// ```
/// `CSVAble` is a trait that a struct can implement when it wants to serialize
/// data in a specific way which can't be accomplished using serde
trait CSVAble {
    /// Given a struct that implements the `CSVAble` trait, this should convert
    /// the fields to be written to a csv file to String and return a vector with
    /// those fields
    fn vectorize(&self) -> Vec<String>;
    /// Given a struct that implements the `CSVAble` trait, this should convert
    /// the field names of the fields to be written to a csv file to String
    /// and return a vector with those fields
    fn get_headers(&self) -> Vec<String>;
}

/// `SubscribeInfo` is a struct which contains the necessary information to send
/// to the finnhub ws api, to subscribe to a stock symbol.
///
/// # Example
/// ```
/// use finnhub_ws::SubscribeInfo;
/// let info: SubscribeInfo = SubscribeInfo::new("AAPL");
/// ```
#[derive(Deserialize, Serialize, Debug)]
pub struct SubscribeInfo<'a> {
    #[serde(rename = "type")]
    pub message_type: &'a str,
    #[serde(rename = "symbol")]
    pub stock_symbol: &'a str,
}

/// `TickerInfo` is a struct which represents the payload of successful response's data
/// of the finnhub api as it can be seen [here](https://finnhub.io/docs/api/websocket-trades).
#[derive(Deserialize, Serialize, Debug)]
pub struct TickerInfo {
    /// symbol: represents the stock symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// price: represents the price of the stock at ticker's time
    #[serde(rename = "p")]
    price: f64,
    /// volume: represents number of stocks of the transaction
    #[serde(rename = "v")]
    volume: f64,
    /// time: represents the time of the transaction and is a millisecond epoch
    #[serde(with = "ts_milliseconds", rename = "t")]
    time: DateTime<Utc>,
    /// conditions: represents the type of transaction or comments
    /// a list of possible values can be found
    /// [here](https://docs.google.com/spreadsheets/d/1PUxiSWPHSODbaTaoL2Vef6DgU-yFtlRGZf19oBb9Hp0/edit#gid=0)
    #[serde(rename = "c")]
    conditions: Option<Vec<String>>,
}

/// `Response` is a struct which represents the successful response of the finnhub api
/// as it can be seen [here](https://finnhub.io/docs/api/websocket-trades).
#[derive(Deserialize, Serialize, Debug)]
pub struct Response<'a> {
    #[serde(rename = "type")]
    pub transaction_type: &'a str,
    #[serde(rename = "data")]
    pub transaction_data: Vec<TickerInfo>,
}

/// `Ping` is a struct which represents the ping action which may be the response
/// obtained by the finnhub api.
#[derive(Deserialize, Debug, Serialize)]
pub struct Ping<'a> {
    #[serde(rename = "type")]
    pub action_type: &'a str
}

/// `Ping` is a struct which represents an error which may be obtained by the finnhub api.
#[derive(Deserialize, Debug, Serialize)]
pub struct WsError<'a> {
    #[serde(rename = "msg")]
    pub message: &'a str
}

/// `WsMessage` represents the three possible response types from the finnhub api for the
/// deserialization if the response to always be valid
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "lowercase")]
pub enum WsMessage<'a> {
    /// The response type to be used on getting data successfully
    #[serde(borrow = "'a")]
    Response(Response<'a>),
    /// The response type to be used when the finnhub api returns an error
    /// Errors may be thrown when using a stock symbol that you don't have
    /// access to or when your token is already being used by another process.
    #[serde(borrow = "'a")]
    Error(WsError<'a>),
    /// The response type be used when the finnhub api is just pinging to check
    /// if the client is still alive.
    #[serde(borrow = "'a")]
    Ping(Ping<'a>)
}

/// `RollingData`: represents the data structure which is being used to serialize
/// and deserialize the transaction data being written to file as they arrive
/// from finnhub.io
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RollingData {
    /// stock symbol of transaction being written
    pub symbol: String,
    /// price of the stock symbol at the time of transaction
    pub price: f64,
    /// timestamp with millisecond precision of the transaction
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    /// timestamp with millisecond precision of writing the transaction to file
    #[serde(with = "ts_milliseconds")]
    pub write_timestamp: DateTime<Utc>
}

impl TickerInfo {
    pub fn default() -> Self {
        TickerInfo {
            symbol: "".parse().unwrap(),
            price: 0.0,
            volume: 0.0,
            time: Utc::now(),
            conditions: Some(vec![]),
        }
    }

    /// Given the stock symbol, its price, the time of transaction and its conditions,
    /// creates and returns a new instance of TickerInfo
    /// # Arguments
    /// - symbol : a string representing the stock symbol
    /// - price: a float representing the price of the stock
    /// - date_time: a UTC datetime representing the time of the transaction
    /// - conditions: a vector of strings which represents the
    ///   type of transaction or comments
    /// # Example
    /// ```
    /// use chrono::{DateTime, TimeZone, Utc};
    /// use finnhub_ws::TickerInfo;
    /// let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
    /// let conditions: Vec<String> = vec!["".parse().unwrap()];
    /// let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
    /// assert_eq!(ticker.symbol, "BINANCE:BTCUSDT");
    /// ```
    pub fn new(symbol: &str, price: f64, volume: f64, date_time: &DateTime<Utc>, conditions: &[String]) -> Self {
        TickerInfo {
            symbol: symbol.parse().unwrap(),
            price,
            volume,
            time: *date_time,
            conditions: Some(conditions.to_vec()),
        }
    }

    /// Method used to persist the ticker to file for analysis later. This serializes the structure
    /// instance to string using the vectorize method and writes that to the csv.
    /// # Arguments
    /// - file: A reference to the rolling file of that stock symbol
    ///
    /// # Example
    /// ```
    /// use std::fs::OpenOptions;
    /// use std::io::{Seek, SeekFrom};
    /// use chrono::{DateTime, TimeZone, Utc};
    /// use finnhub_ws::TickerInfo;
    /// let mut file = OpenOptions::new()
    ///        .write(true)
    ///        .append(true)
    ///        .create(true)
    ///        .read(true)
    ///        .open("BINANCE_BTCUSDT.csv").unwrap();
    /// let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
    /// let conditions: Vec<String> = vec!["".parse().unwrap()];
    /// let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
    /// ticker.write_to_disk(&file);
    /// std::fs::remove_file("BINANCE_BTCUSDT.csv").unwrap();
    pub fn write_to_disk(&self, file: &File) {
        let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(file);
        writer.serialize(self.vectorize()).unwrap();
        writer.flush().unwrap();
    }


    /// Method used to check if the file given as input is empty or not. It returns true when the file is empty
    /// and false otherwise. It tries to open the file as a csv, reads the first line and if the result can be
    /// read, the file is not empty
    /// # Arguments
    /// - file: a reference to the file to be checked
    ///
    /// # Examples
    /// ```
    /// use std::fs::OpenOptions;
    /// use chrono::{DateTime, TimeZone, Utc};
    /// use finnhub_ws::TickerInfo;
    /// let mut file = OpenOptions::new()
    ///        .write(true)
    ///        .append(true)
    ///        .create(true)
    ///        .read(true)
    ///        .open("BINANCE_BTCUSDT.csv").unwrap();
    /// let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
    /// let conditions: Vec<String> = vec!["".parse().unwrap()];
    /// let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
    /// assert_eq!(ticker.check_file_empty(&file), true);
    /// std::fs::remove_file("BINANCE_BTCUSDT.csv").unwrap();
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
    /// Method, implementation of CSVAble trait. Serializes the necessary TickerInfo fields
    /// and returns a vector of stringified fields.
    fn vectorize(&self) -> Vec<String> {
        return vec![self.symbol.clone(),
                    self.price.to_string(),
                    self.time.timestamp_millis().to_string(),
                    Utc::now().timestamp_millis().to_string(),
        ];
    }

    /// Returns a vector of strings containing the field names of the serialized data.
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
    /// Given a string slice containing the stock to track,
    /// this creates the necessary struct instance to send
    /// to the finnhub api
    ///
    /// # Example
    /// ```
    /// use finnhub_ws::SubscribeInfo;
    /// let stock_info = SubscribeInfo::new("AAPL");
    /// assert_eq!(stock_info.stock_symbol, "AAPL");
    /// assert_eq!(stock_info.message_type, "subscribe");
    pub fn new(symbol: &'a str) -> Self {
        SubscribeInfo {
            message_type: "subscribe",
            stock_symbol: symbol,
        }
    }
}


#[cfg(test)]
mod finnhub_ws_lib_test {
    use std::fs::OpenOptions;
    use std::io::{Read, Seek, SeekFrom};
    use chrono::{DateTime, DurationRound, TimeZone, Utc};
    use crate::{CSVAble, TickerInfo};
    use serial_test::serial;

    #[test]
    fn given_arguments_should_create_valid_ticker_info(){
        let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
        let conditions: Vec<String> = vec!["".parse().unwrap()];
        let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
        assert_eq!(ticker.symbol, "BINANCE:BTCUSDT");
    }

    #[test]
    fn should_create_default_ticker_info(){
        let ticker = TickerInfo::default();
        println!("{}", ticker.time);
        assert_eq!(ticker.symbol, "");
        assert_eq!(ticker.conditions, Some(vec![]));
        assert_eq!(ticker.volume, 0.0);
        assert_eq!(ticker.price, 0.0);
        assert_eq!(ticker.time.duration_trunc(chrono::Duration::minutes(1)).unwrap(), Utc::now().duration_trunc(chrono::Duration::minutes(1)).unwrap())
    }

    #[test]
    fn given_a_ticker_info_instance_should_write_to_file(){
        let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .read(true)
                .open("given_a_ticker_info_instance_should_write_to_file.csv").unwrap();
        let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
        let conditions: Vec<String> = vec![];
        let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
        ticker.write_to_disk(&file);
        let mut data = String::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_string(&mut data).unwrap();
        data.truncate(data.len() -1 );
        let got = data.split(',').collect::<Vec<&str>>();
        assert_eq!(got[0..got.len()-1], ticker.vectorize()[0..got.len()-1]);
        std::fs::remove_file("given_a_ticker_info_instance_should_write_to_file.csv").unwrap();
    }

    #[test]
    fn given_a_non_empty_file_check_if_file_is_empty(){
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open("given_a_non_empty_file_check_if_file_is_empty.csv").unwrap();
        let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
        let conditions: Vec<String> = vec![];
        let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
        ticker.write_to_disk(&file);
        file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(ticker.check_file_empty(&file), false);
        std::fs::remove_file("given_a_non_empty_file_check_if_file_is_empty.csv").unwrap();
    }

    #[test]
    fn given_an_empty_file_check_if_file_is_empty(){
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open("given_an_empty_file_check_if_file_is_empty.csv").unwrap();
        let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
        let conditions: Vec<String> = vec![];
        let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
        assert_eq!(ticker.check_file_empty(&file), true);
        std::fs::remove_file("given_an_empty_file_check_if_file_is_empty.csv").unwrap();
    }

    #[test]
    fn given_a_ticker_it_should_write_csv_headers_to_file(){
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open("given_a_ticker_it_should_write_csv_headers_to_file.csv").unwrap();
        let date: DateTime<Utc> = Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376);
        let conditions: Vec<String> = vec![];
        let ticker = TickerInfo::new("BINANCE:BTCUSDT", 23841.51, 1.0, &date, &conditions );
        ticker.write_headers(&file);
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        data.truncate(data.len() - 1);
        let got: Vec<&str> = data.split(',').collect::<Vec<&str>>();
        assert_eq!(got, ticker.get_headers());
        std::fs::remove_file("given_a_ticker_it_should_write_csv_headers_to_file.csv").unwrap();
    }

    #[test]
    fn given_a_ticker_it_should_serialize_it(){

    }
}