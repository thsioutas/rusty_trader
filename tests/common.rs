use chrono::{Duration, NaiveDateTime};
use std::io::Write;
use tempfile::NamedTempFile;

pub fn generate_backtest_csv() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "timestamp,price,volume").unwrap();

    let start = NaiveDateTime::parse_from_str("2023-01-01 09:30:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..400 {
        let ts = start + Duration::minutes(i as i64);
        let price = if i < 200 {
            100.0 + i as f64 * 0.2 // uptrend
        } else {
            200.0 - (i as f64 - 200.0) * 0.3 // downtrend
        };
        writeln!(file, "{},{},{}", ts, price, 1500).unwrap();
    }
    file
}
