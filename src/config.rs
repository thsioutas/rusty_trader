use std::collections::HashMap;

use anyhow::Result;
use config::Value;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    /// Connections to Interactive Broker instances
    #[serde(default)]
    pub ib_connections: Vec<IbConnectionConfig>,
    pub brokers: Vec<BrokerConfig>,
    pub strategies: Vec<StrategyConfig>,
    pub data_feeds: Vec<DataFeedConfig>,
}

#[derive(Debug, Deserialize)]
pub struct IbConnectionConfig {
    pub name: String,
    pub address: String,
    pub client_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct BrokerConfig {
    pub name: String,
    pub r#type: BrokerType,
    pub params: Option<HashMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub struct StrategyConfig {
    /// The name of the strategy
    pub name: String,
    /// The type of the strategy
    pub r#type: StrategyType,
    /// The broker that should be used for the specific strategy
    pub broker: String,
    /// The data feed that should be used for the specific strategy
    pub data_feed: String,
    /// Extra optional parameters that might be needed for the specific strategy
    pub params: Option<HashMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub enum BrokerType {
    DummyBroker,
    IbBroker,
}

#[derive(Debug, Deserialize)]
pub enum StrategyType {
    PrintStrategy,
    SmaCrossStrategy,
}

#[derive(Debug, Deserialize)]
pub struct DataFeedConfig {
    pub name: String,
    pub r#type: DataFeedType,
    pub symbol: String,
    pub params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub enum DataFeedType {
    CsvDataFeed,
    IbMarketDataFeed,
    IbHistoricalDataFeed,
}

impl BotConfig {
    pub fn deserialize_from_file(path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?;
        let config = config.try_deserialize()?;
        Ok(config)
    }
}
