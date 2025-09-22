use super::DataFeed;
use crate::data_feed::MarketData;
use async_trait::async_trait;
use csv::ReaderBuilder;
use std::{collections::VecDeque, fs::File};
use thiserror::Error;

pub struct CsvDataFeed {
    name: String,
    data: VecDeque<MarketData>,
}

impl CsvDataFeed {
    pub fn new(name: String, path: String) -> Result<Self, CsvDataFeedError> {
        let file = File::open(&path)
            .map_err(|err| CsvDataFeedError::FileOpenError(path.clone(), err.to_string()))?;
        let mut rdr = ReaderBuilder::new().from_reader(file);
        let mut data = VecDeque::new();
        for result in rdr.deserialize() {
            // TODO: Handle error
            let record: (String, f64) = result.unwrap();
            let md = MarketData {
                symbol: "AAPL".to_string(),
                price: record.1,
                // timestamp:
            };
            data.push_back(md);
        }
        Ok(Self { name, data })
    }
}

#[async_trait]
impl DataFeed for CsvDataFeed {
    fn name(&self) -> &str {
        &self.name
    }
    async fn next_tick(&mut self) -> Option<MarketData> {
        self.data.pop_front()
    }
}

#[derive(Debug, Error)]
pub enum CsvDataFeedError {
    #[error("Failed to open CSV file ({0}): {1}")]
    FileOpenError(String, String),
}
