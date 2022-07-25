use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Seek, SeekFrom};
use std::sync::{Arc, Mutex, Once};
use chrono::{Utc};
use crossbeam_channel::{Receiver, Sender, unbounded};
use crate::candlestick::Candlestick;
use crate::TickerInfo;
use crate::utils::sanitize_string;

#[derive(Debug)]
pub struct StockHandle {
    pub stock_symbol: String,
    pub rolling_file: Mutex<File>,
    pub candlestick_file: Mutex<File>,
    pub mean_file: Mutex<File>,
    pub once_flag: Once,
    pub stock_channel: (Sender<i64>, Receiver<i64>),
    pub rolling_mean_channel: (Sender<i64>, Receiver<i64>)
}

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



