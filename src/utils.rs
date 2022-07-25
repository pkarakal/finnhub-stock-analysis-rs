use std::fs::{create_dir_all, File};
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use serde::{Deserialize};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use crate::{RollingData};

pub fn sanitize_string(s: &str) -> String {
    let re = Regex::new(r"\W").unwrap();
    re.replace_all(s, "_").to_string()
}

pub fn create_dirs(p: &str) -> bool {
    match create_dir_all(p) {
        Ok(_) => true,
        Err(e) => {
            match e.kind() {
                io::ErrorKind::PermissionDenied => {
                    eprintln!("Cannot create directory due to permission errors");
                    false
                }
                io::ErrorKind::AlreadyExists => true,
                _ => false,
            }
        }
    }
}

pub fn is_file_empty(f: File) -> bool {
    let mut reader = BufReader::new(f);
    let mut data = Vec::new();
    let l = reader.read_to_end(&mut data).expect("stream did not contain valid UTF-8");
    println!("{:?} {:?}", data, l);
    !!data.is_empty()
}

pub fn find_items(file: &mut File, time: i64, l: i64) -> Vec<RollingData> {
    let datetime_min: DateTime<Utc> = DateTime::from_utc(NaiveDateTime::from_timestamp(time, 0), Utc);
    let datetime_max: DateTime<Utc> = datetime_min + chrono::Duration::minutes(l);
    let mut data:String = String::new();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.read_to_string(&mut data).unwrap();
    let mut records: Vec<RollingData> = Vec::with_capacity(100);
    let mut reader = csv::ReaderBuilder::new().from_reader(data.as_bytes());
    for record in reader.deserialize(){
        let record: RollingData = record.unwrap();
        if record.write_timestamp.ge(&datetime_min) && record.write_timestamp.lt(&datetime_max){
            records.push(record);
        }
    }
    records
}


#[cfg(test)]
mod utils_test {
    use crate::utils::{create_dirs, find_items, is_file_empty, sanitize_string};
    use std::fs::{File, OpenOptions, remove_dir_all, remove_file};
    use std::io::{Read, Write};
    use chrono::{TimeZone, Utc};
    use serial_test::serial;
    use crate::RollingData;


    fn create_file(s: &str) -> File {
        OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .read(true)
            .open(s).unwrap()
    }

    #[test]
    fn check_sanitize_string_with_semi_colons() {
        let string = "BINANCE:BTCUSDT";
        let got = sanitize_string(string);
        assert_eq!(got, "BINANCE_BTCUSDT".to_string())
    }

    #[test]
    fn check_sanitize_string_with_special_char() {
        let string = "M$FT";
        let got = sanitize_string(string);
        assert_eq!(got, "M_FT".to_string())
    }

    #[test]
    fn check_create_dirs() {
        let path = "./tmp/data";
        let got = create_dirs(path);
        assert_eq!(got, true);
        remove_dir_all(path).unwrap();
    }

    #[test]
    fn try_create_dir_in_root() {
        let path = "/finnhub-ws";
        let got = create_dirs(path);
        assert_eq!(got, false)
    }

    #[test]
    #[serial]
    fn check_if_non_empty_file_is_empty() {
        let file_name = "test/non_empty.txt";
        create_dirs("test");
        let mut file = create_file(file_name);
        file.write_all(b"this is not an empty file").unwrap();
        // TODO: implement the test in a sane manner.
        drop(file);
        let file = create_file(file_name);
        let got = is_file_empty(file);
        assert_eq!(got, false);
        remove_file(file_name).unwrap();
        remove_dir_all("test").unwrap();
    }

    #[test]
    #[serial]
    fn check_if_actually_empty_file_is_empty() {
        let file_name = "test/empty.txt";
        create_dirs("test");
        let file = create_file(file_name);
        let got = is_file_empty(file);
        assert_eq!(got, true);
        remove_file(file_name).unwrap();
        remove_dir_all("test").unwrap();
    }

    fn write_to_file(f: &mut File) {
        f.write(b"Symbol,Price,Timestamp,WriteTimestamp
BINANCE:BTCUSDT,23061.05,1658441258376,1658441270794
BINANCE:BTCUSDT,23060.16,1658441258197,1658441270794
BINANCE:BTCUSDT,23061.04,1658441258362,1658441270795
BINANCE:BTCUSDT,23060.88,1658441258330,1658441270797
BINANCE:BTCUSDT,23061.05,1658441258362,1658441270797
BINANCE:BTCUSDT,23060.89,1658441258340,1658441270814
BINANCE:BTCUSDT,23058.59,1658441258404,1658441271008
BINANCE:BTCUSDT,23061.79,1658441258466,1658441271009").unwrap();
    }

    pub fn find_items_test(mut file: File, time: i64, l: i64) -> Vec<RollingData> {
        let d: &mut String = &mut "".to_string();
        file.read_to_string(d).unwrap();
        let mut reader = csv::ReaderBuilder::new().from_reader(d.as_bytes());
        let mut records: Vec<RollingData> = vec![];
        for item in  reader.records(){
            let record= item.unwrap();
            records.push(RollingData {
                symbol: record[0].parse().unwrap(),
                price: record[1].parse().unwrap(),
                timestamp: Utc.timestamp_millis(record[2].parse().unwrap()),
                write_timestamp: Utc.timestamp_millis(record[3].parse().unwrap())
            });
        }
        records
    }

    #[test]
    #[serial]
    fn write_to_disk() {
        let file_name = "test/test.csv";
        create_dirs("test");
        let mut file = create_file(file_name);
        write_to_file(&mut file);
        let got = find_items(&mut file, 1658441258, 1);
        let expected = vec![
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.05, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23060.16, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 197), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.04, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 362), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 795) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23060.88, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 330), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 797) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.05, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 362), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 797) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23060.89, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 340), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 814) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23058.59, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 404), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 51, 8) },
            RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.79, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 466), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 51, 9) },
        ];
        assert_eq!(got, expected);
        remove_file(file_name).unwrap();
        remove_dir_all("test").unwrap();
    }
}