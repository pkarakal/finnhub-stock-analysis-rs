use std::fs::{create_dir_all, File};
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use serde::{Deserialize};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use crate::{RollingData};

/// Given a string containing special characters, it will return the original
/// string with an _underscore_ instead of the special characters
///
/// # Arguments
/// * `s` - A string slice that holds a string with special characters
///
/// # Examples
/// ```
/// use finnhub_ws::utils::sanitize_string;
/// let name = sanitize_string("BINANCE:BTCUSDT");
/// assert_eq!("BINANCE_BTCUSDT", name);
/// ```
pub fn sanitize_string(s: &str) -> String {
    let re = Regex::new(r"\W").unwrap();
    re.replace_all(s, "_").to_string()
}

/// Given a directory path, this creates the necessary directories that
/// result from the string. It returns true when the directories already
/// exist or have been successfully created and false whenever the user
/// executing the program doesn't have the necessary permissions or in
/// any other error that may occur during `create_dir_all`
///
/// # Arguments
/// - `p` -  A string slice that holds the path to be created
///
/// # Example
/// ```
/// use finnhub_ws::utils::create_dirs;
/// let created = create_dirs("tmp");
/// assert_eq!(created, true);
/// ```
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

/// Given a file, this returns whether it is empty or not by reading it to the end.
/// If the buffer is empty, it returns true, and false otherwise. An error may occur
/// if the file doesn't contain valid UTF-8 characters
///
/// # Arguments
/// `f` - A file descriptor pointing to the file to be tested
///
/// # Example
/// ```
/// use std::fs::{File, OpenOptions};
/// use tokio::fs::remove_file;
/// use finnhub_ws::utils::{is_file_empty, create_dirs};
/// let _ = create_dirs("tmp");
/// let f = OpenOptions::new()
///     .write(true)
///     .append(true)
///     .create(true)
///     .read(true)
///     .open("tmp/file").unwrap();
/// let is_empty = is_file_empty(f);
/// assert_eq!(is_empty, true);
/// std::fs::remove_file("tmp/file").unwrap();
/// ```
pub fn is_file_empty(f: File) -> bool {
    let mut reader = BufReader::new(f);
    let mut data = Vec::new();
    let l = reader.read_to_end(&mut data).expect("stream did not contain valid UTF-8");
    println!("{:?} {:?}", data, l);
    !!data.is_empty()
}

/// Given a file, a timestamp and a delta of time, it returns all matching records from the file.
/// It checks if the records where written to the file between the given timestamp and the
/// timestamp + delta.
///
/// # Arguments
/// - `file` - A mutable reference to a file from which the records should be obtained. The mutability here
///            is necessary to seek back to the start of the file
/// - `time` - A datetime timestamp given in milliseconds
/// - `l` - The delta of time between the two timestamps to search
///
/// # Example
/// ```
/// use std::io::Write;
/// use chrono::{TimeZone, Utc};
/// use tokio::fs::remove_file;
/// use finnhub_ws::utils::{find_items, create_dirs};
/// let _ = create_dirs("tmp");
/// let mut f = std::fs::OpenOptions::new()
///     .write(true)
///     .append(true)
///     .create(true)
///     .read(true)
///     .open("tmp/find_items.csv").unwrap();
/// f.write(b"Symbol,Price,Timestamp,WriteTimestamp
/// BINANCE:BTCUSDT,23061.05,1658441258376,1658441270794
/// BINANCE:BTCUSDT,23060.16,1658441258197,1658441270794
/// BINANCE:BTCUSDT,23061.04,1658441258362,1658441270795").unwrap();
/// f.sync_all().unwrap();
/// let items = find_items(&mut f, 1658441258, 1);
/// assert_eq!(items, vec![///
///     finnhub_ws::RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.05, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 376), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794) },
///     finnhub_ws::RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23060.16, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 197), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 794) },
///     finnhub_ws::RollingData { symbol: "BINANCE:BTCUSDT".parse().unwrap(), price: 23061.04, timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 38, 362), write_timestamp: Utc.ymd(2022, 7, 21).and_hms_milli(22, 7, 50, 795) },
/// ]);
/// std::fs::remove_file("tmp/find_items.csv").unwrap();
/// ```
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
    fn given_a_string_with_no_special_char_expect_same_string() {
        let string = "Hello";
        let got = sanitize_string(string);
        assert_eq!(got, "Hello".to_string())
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

    fn write_mock_data_to_file(f: &mut File) {
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

    #[test]
    #[serial]
    fn given_file_with_data_expect_vec_with_matching_records() {
        let file_name = "test/test.csv";
        create_dirs("test");
        let mut file = create_file(file_name);
        write_mock_data_to_file(&mut file);
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

    #[test]
    #[serial]
    fn given_empty_file_expect_empty_vec() {
        let file_name = "test/test_empty.csv";
        create_dirs("test");
        let mut file = create_file(file_name);
        let got = find_items(&mut file, 1658441258, 1);
        let expected = vec![];
        assert_eq!(got, expected);
        remove_file(file_name).unwrap();
        remove_dir_all("test").unwrap();
    }

    #[test]
    #[serial]
    fn given_future_timestamp_expect_empty_vec() {
        let file_name = "test/test.csv";
        create_dirs("test");
        let mut file = create_file(file_name);
        write_mock_data_to_file(&mut file);
        let got = find_items(&mut file, 1658860842, 1);
        let expected = vec![];
        assert_eq!(got, expected);
        remove_file(file_name).unwrap();
        remove_dir_all("test").unwrap();
    }
}