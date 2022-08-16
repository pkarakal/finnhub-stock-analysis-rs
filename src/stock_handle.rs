//! Stock handle primitives
//! # stock_handle
//!
//! This contains the necessary structure and functions to manage
//! all the needs the program has with regard to stock symbols tracked
//! open file descriptors and channels.
//!
//! # Example
//! ```
//! use finnhub_ws::stock_handle::initialize_mapper;
//! let stock_handle = initialize_mapper(&["AAPL".to_string()]);
//! assert_eq!(stock_handle.len(), 1);
//! assert_eq!(stock_handle[0].stock_symbol, "AAPL".to_string());
//! ```
use std::fs::{File, OpenOptions};
use std::io;
use std::sync::{Arc, Mutex, Once};
use chrono::{Utc};
use crossbeam_channel::{Receiver, Sender, unbounded};
use crate::candlestick::Candlestick;
use crate::TickerInfo;
use crate::utils::sanitize_string;

/// `StockHandle` holds all the necessary data to manage a stock symbol
/// such as any open file descriptors for the rolling, mean and candlestick
/// information, and the channels for the threads to be able to send timestamps
/// to calculate mean and candlestick data.
#[derive(Debug)]
pub struct StockHandle {
    /// The symbol the stock has in the trade market.
    /// It can also be an exchange like EUR/USD
    pub stock_symbol: String,
    /// The file descriptor where the trade information
    /// gets written to as they arrive. This has a mutex so that it can
    /// be easily passed between threads and avoid data races
    /// or parsing errors due to file being read while being written
    pub rolling_file: Mutex<File>,
    /// The file descriptor where the candlestick information
    /// gets written to each minute. This has a mutex so that it can
    /// be easily passed between threads and avoid data races
    /// or parsing errors due to file being read while being written
    pub candlestick_file: Mutex<File>,
    /// The file descriptor where the mean price information
    /// gets written to each minute. This has a mutex so that it can
    /// be easily passed between threads and avoid data races
    /// or parsing errors due to file being read while being written
    pub mean_file: Mutex<File>,
    /// The once flag is a synchronization primitive to ensure that
    /// the headers get written to the file just once and that block
    /// of code gets run only once during initialization.
    pub once_flag: Once,
    /// `stock_channel` holds a tuple of Sender and receiver of i64
    /// timestamps. This is the primary way of communicating between
    /// the producing thread and the consumer ones. Each minute a
    /// millis timestamp gets written to the sender and received by
    /// the receiver. Upon receiving the data, the candlestick should
    /// be calculated
    pub stock_channel: (Sender<i64>, Receiver<i64>),
    /// `rolling_mean_channel` holds a tuple of Sender and receiver of i64
    /// timestamps. This is the primary way of communicating between
    /// the producing thread and the consumer ones. Evey 15 minute a
    /// millis timestamp gets written to the sender and received by
    /// the receiver. Upon receiving the data, the mean price should
    /// be calculated
    pub rolling_mean_channel: (Sender<i64>, Receiver<i64>)
}

/// Given a string slice containing the stock symbol in the trade market,
/// it returns a file descriptor if it was successful in opening or creating it.
/// The file will be located under data/rolling directory and be named as
/// {sanitized_stock_symbol}.csv
///
/// # Arguments
/// `stock` - A string slice containing the stock symbol
///
/// # Example
/// ```
/// use finnhub_ws::stock_handle::create_rolling_file;
/// let f = create_rolling_file("TSLA").unwrap();
/// ```
pub fn create_rolling_file(stock: &str) -> Option<File> {
    let safe_stock = sanitize_string(stock);
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .read(true)
        .open(format!("data/rolling/{}.csv", safe_stock)) {
        Ok(f) => Some(f),
        Err(err) => match err.kind() {
            io::ErrorKind::PermissionDenied => {
                eprintln!("Cannot create a file due to permission reasons");
                None
            }
            _ => {
                eprintln!("Couldn't create file");
                None
            }
        }
    }
}

/// Given a string slice containing the stock symbol in the trade market,
/// it returns a file descriptor if it was successful in opening or creating it.
/// The file will be located under data/candlestick directory and be named as
/// {sanitized_stock_symbol}.csv

/// # Arguments
/// `stock` - A string slice containing the stock symbol
///
/// # Example
/// ```
/// use finnhub_ws::stock_handle::create_rolling_file;
/// let f = create_rolling_file("TSLA").unwrap();
/// ```
pub fn create_candlestick_file(stock: &str) -> Option<File> {
    let safe_stock = sanitize_string(stock);
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .read(true)
        .open(format!("data/candlestick/{}.csv", safe_stock)) {
        Ok(f) => Some(f),
        Err(err) => match err.kind() {
            io::ErrorKind::PermissionDenied => {
                eprintln!("Cannot create a file due to permission reasons");
                None
            }
            _ => {
                eprintln!("Couldn't create file");
                None
            }
        }
    }
}
/// Given a string slice containing the stock symbol in the trade market,
/// it returns a file descriptor if it was successful in opening or creating it.
/// The file will be located under data/mean directory and be named as
/// {sanitized_stock_symbol}.csv

/// # Arguments
/// `stock` - A string slice containing the stock symbol
///
/// # Example
/// ```
/// use finnhub_ws::stock_handle::create_rolling_file;
/// let f = create_rolling_file("TSLA").unwrap();
/// ```
pub fn create_mean_file(stock: &str) -> Option<File> {
    let safe_stock = sanitize_string(stock);
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .read(true)
        .open(format!("data/mean/{}.csv", safe_stock)) {
        Ok(f) => Some(f),
        Err(err) => match err.kind() {
            io::ErrorKind::PermissionDenied => {
                eprintln!("Cannot create a file due to permission reasons");
                None
            }
            _ => {
                eprintln!("Couldn't create file");
                None
            }
        }
    }
}

/// Given an array of strings containing the stocks to track, it returns
/// an atomically reference counted vector of `StockHandle`s. It creates
/// the necessary files and wraps them around a mutex, creates the channels
/// and runs once the writing of headers to those files.
///
/// # Arguments
/// `stocks` : reference of array of strings containing the stocks being tracked.
///
/// # Example
/// ```
/// use finnhub_ws::stock_handle::initialize_mapper;
/// let mapper = initialize_mapper(&["AAPL".to_string(), "BINANCE:BTCUSDT".to_string()]);
/// assert_eq!(mapper.len(), 2);
/// ```
pub fn initialize_mapper(stocks: &[String])-> Arc<Vec<StockHandle>>{
    let mut mapper = Vec::with_capacity(stocks.len());
    stocks.iter().for_each(|x| {
        let rolling = create_rolling_file(x.as_str()).unwrap();
        let candlestick = create_candlestick_file(x.as_str()).unwrap();
        let mean = create_mean_file(x.as_str()).unwrap();
        let res = StockHandle{
            stock_symbol: x.to_string(),
            rolling_file: Mutex::new(rolling),
            candlestick_file: Mutex::new(candlestick),
            mean_file: Mutex::new(mean),
            once_flag: Once::new(),
            stock_channel: unbounded(),
            rolling_mean_channel: unbounded()
        };
        res.once_flag.call_once(||{
            let t = TickerInfo::default();
            let rf = res.rolling_file.lock().unwrap();
            if t.check_file_empty(&rf) {
                t.write_headers(&rf);
            }
            drop(rf);
            let cf = res.candlestick_file.lock().unwrap();
            let c = Candlestick{
                open_price: 0.0,
                close_price: 0.0,
                highest_price: 0.0,
                lowest_price: 0.0,
                total_transactions: 0,
                minute_of_hour: Utc::now(),
                stock_symbol: "".parse().unwrap()
            };
            c.write_to_file(&cf);
            drop(cf);
        });
        mapper.push(res);
    });
    Arc::new(mapper)
}


#[cfg(test)]
mod stock_handle_test {
    use std::fs::remove_file;
    use std::ops::Deref;
    use std::sync::Arc;
    use crate::stock_handle::{create_candlestick_file, create_mean_file, create_rolling_file, initialize_mapper, StockHandle};
    use crate::utils::{create_dirs, sanitize_string};

    #[test]
    fn given_a_stock_symbol_it_should_create_rolling_file() {
        let _ = create_dirs("data/rolling");
        let stock_name = "rolling";
        let f = create_rolling_file(stock_name).unwrap();
        drop(f);
        let file_exists = std::fs::metadata("data/rolling/rolling.csv").unwrap();
        assert_eq!(file_exists.is_file(), true);
        remove_file("data/rolling/rolling.csv").unwrap();
    }

    #[test]
    fn given_a_stock_symbol_it_should_create_candlestick_file() {
        let _ = create_dirs("data/candlestick");
        let stock_name = "candlestick";
        let f = create_candlestick_file(stock_name).unwrap();
        drop(f);
        let file_exists = std::fs::metadata("data/candlestick/candlestick.csv").unwrap();
        assert_eq!(file_exists.is_file(), true);
        remove_file("data/candlestick/candlestick.csv").unwrap();
    }

    #[test]
    fn given_a_stock_symbol_it_should_create_mean_file() {
        let _ = create_dirs("data/mean");
        let stock_name = "mean";
        let f = create_mean_file(stock_name).unwrap();
        drop(f);
        let file_exists = std::fs::metadata("data/mean/mean.csv").unwrap();
        assert_eq!(file_exists.is_file(), true);
        remove_file("data/mean/mean.csv").unwrap();
    }

    #[test]
    fn given_an_array_of_stocks_it_should_create_the_mapper_files() {
        for dir in ["data/rolling", "data/mean", "data/candlestick"] {
            let _ = create_dirs(dir);
        }
        let stocks = vec!["abc".to_string(), "def".to_string(), "ghi".to_string()];
        let mapper = initialize_mapper(&stocks);
        assert_eq!(mapper.len(), 3);
        assert_eq!(std::fs::metadata("data/rolling").unwrap().is_dir(), true);
        for stock in stocks {
            assert_eq!(std::fs::metadata(format!("data/rolling/{}.csv", sanitize_string(&stock) )).unwrap().is_file(), true);
            assert_eq!(std::fs::metadata(format!("data/candlestick/{}.csv",sanitize_string(&stock) )).unwrap().is_file(), true);
            assert_eq!(std::fs::metadata(format!("data/mean/{}.csv", sanitize_string(&stock) )).unwrap().is_file(), true);
            remove_file(format!("data/rolling/{}.csv", sanitize_string(&stock))).unwrap();
            remove_file(format!("data/candlestick/{}.csv", sanitize_string(&stock))).unwrap();
            remove_file(format!("data/mean/{}.csv", sanitize_string(&stock))).unwrap();
        }
    }

    #[test]
    fn given_a_stock_symbol_it_should_create_the_mapper_channels(){
        for dir in ["data/rolling", "data/mean", "data/candlestick"] {
            let _ = create_dirs(dir);
        }
        let stocks = vec!["jkl".to_string()];
        let mapper = initialize_mapper(&stocks);
        let handles: &Vec<StockHandle> = mapper.deref();
        for handle in handles {
            let (tx,rx) = &handle.stock_channel;
            tx.send(1234).unwrap();
            assert_eq!(rx.recv().unwrap(), 1234);
            remove_file(format!("data/rolling/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
            remove_file(format!("data/candlestick/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
            remove_file(format!("data/mean/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
        }
    }

    #[test]
    fn given_a_stock_symbol_it_should_create_the_mapper_mean_channels(){
        for dir in ["data/rolling", "data/mean", "data/candlestick"] {
            let _ = create_dirs(dir);
        }
        let stocks = vec!["mno".to_string()];
        let mapper = initialize_mapper(&stocks);
        let handles: &Vec<StockHandle> = mapper.deref();
        for handle in handles {
            let (tx,rx) = &handle.rolling_mean_channel;
            tx.send(1234).unwrap();
            assert_eq!(rx.recv().unwrap(), 1234);
            remove_file(format!("data/rolling/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
            remove_file(format!("data/candlestick/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
            remove_file(format!("data/mean/{}.csv", sanitize_string(&handle.stock_symbol))).unwrap();
        }
    }

}

