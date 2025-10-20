use crate::{
    broker::{Broker, dummy::DummyBroker, ib::Ib},
    config::{
        BotConfig, BrokerConfig, BrokerType, DataFeedConfig, DataFeedType, IbConnectionConfig,
        PositionSizerConfig, PositionSizerType, StrategyType,
    },
    data_feed::{
        DataFeed, csv_data_feed::CsvDataFeed, ib_historical_data_feed::IbHistoricalDataFeed,
        ib_market_data_feed::IbMarketDataFeed,
    },
    position_sizer::{
        PositionSizer, fixed_sizer::FixedSizer, percent_of_equity_sizer::PercentOfEquitySizer,
    },
    strategy::{
        Strategy,
        print::PrintStrategy,
        sma_cross::{
            DEFAULT_SMA_CROSS_FAST_WINDOW, DEFAULT_SMA_CROSS_SLOW_WINDOW, SmaCrossStrategy,
        },
    },
};
use config::Value;
use ibapi::{
    Client,
    market_data::MarketDataType as IbMarketDataType,
    market_data::historical::{BarSize, Duration, ToDuration},
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use time::{OffsetDateTime, PrimitiveDateTime, macros::format_description};
use tracing::{debug, error, warn};

pub async fn build_strategies(
    bot_config: BotConfig,
) -> Result<Vec<Box<dyn Strategy>>, FactoryError> {
    let ib_connections = build_connections(bot_config.ib_connections)?;
    // TODO: Discuss if this is the best design (via FillListener and a mpsc) for the update of portfolio
    // TODO: Pass an ib_connection or config init data (maybe based on broker type?) to portfolio constructor
    // let fill_listener = FillListener::new(fill_rx, portfolio.clone());
    // fill_listener.start().await;
    let brokers = build_brokers(bot_config.brokers, &ib_connections)?;
    let mut data_feeds = build_data_feeds(bot_config.data_feeds, &ib_connections)?;
    let mut position_sizers = build_sizers(bot_config.position_sizers)?;
    let mut strategies = Vec::new();
    for config in bot_config.strategies {
        // Brokers are shared between strategies. That's why they stay in Arc
        let broker = brokers
            .get(&config.broker)
            .ok_or(FactoryError::UnknownBroker(config.broker))?;
        // Feeds stay at Box because each strategy should own its feed instance (to avoid tick stealing)
        let data_feed = data_feeds
            .remove(&config.data_feed)
            .ok_or(FactoryError::UnknownDataFeed(config.data_feed))?;
        // Sizers stay at Box because each strategy should own its sizer instance.
        let sizer = position_sizers
            .remove(&config.position_sizer)
            .ok_or(FactoryError::UnknownPositionSizer(config.position_sizer))?;
        match config.r#type {
            StrategyType::PrintStrategy => {
                let strategy: Box<dyn Strategy> = Box::new(PrintStrategy {
                    name: config.name,
                    data_feed,
                    broker: broker.clone(),
                    position_sizer: sizer,
                });
                strategies.push(strategy);
            }
            StrategyType::SmaCrossStrategy => {
                let fast_window =
                    get_usize_param(&config.params, "fast_window", DEFAULT_SMA_CROSS_FAST_WINDOW);
                let slow_window =
                    get_usize_param(&config.params, "slow_window", DEFAULT_SMA_CROSS_SLOW_WINDOW);

                let strategy: Box<dyn Strategy> = Box::new(SmaCrossStrategy::new(
                    config.name,
                    data_feed,
                    broker.clone(),
                    sizer,
                    fast_window,
                    slow_window,
                ));
                strategies.push(strategy);
            }
        }
    }
    Ok(strategies)
}

fn build_connections(
    configs: Vec<IbConnectionConfig>,
) -> Result<HashMap<String, Arc<Client>>, FactoryError> {
    let mut ib_connections = HashMap::new();
    for config in configs {
        debug!(
            "Attempting IB connection '{}' at {} (client_id={})",
            config.name, config.address, config.client_id
        );
        let client = Client::connect(&config.address, config.client_id).map_err(|err| {
            FactoryError::IbConnectionFailure(config.name.clone(), err.to_string())
        })?;
        debug!(
            "Established IB connection '{}' at {} (client_id={})",
            config.name, config.address, config.client_id
        );
        ib_connections.insert(config.name, Arc::new(client));
    }
    Ok(ib_connections)
}

fn build_brokers(
    configs: Vec<BrokerConfig>,
    ib_connections: &HashMap<String, Arc<Client>>,
) -> Result<HashMap<String, Arc<dyn Broker>>, FactoryError> {
    let mut brokers = HashMap::new();
    for config in configs {
        let broker: Arc<dyn Broker> = match config.r#type {
            BrokerType::DummyBroker => Arc::new(DummyBroker::new(config.name.clone())),
            BrokerType::IbBroker => {
                let ib_connection = get_ib_connection(config.params.as_ref(), ib_connections)?;
                let ib_broker = Ib::new(config.name.clone(), ib_connection.clone())
                    .map_err(|err| FactoryError::BrokerInit(err.to_string()))?;
                Arc::new(ib_broker)
            }
        };
        brokers.insert(config.name, broker);
    }
    Ok(brokers)
}

fn build_data_feeds(
    configs: Vec<DataFeedConfig>,
    ib_connections: &HashMap<String, Arc<Client>>,
) -> Result<HashMap<String, Box<dyn DataFeed>>, FactoryError> {
    let mut data_feeds = HashMap::new();
    for config in configs {
        let data_feed: Box<dyn DataFeed> = match config.r#type {
            DataFeedType::CsvDataFeed => {
                let path_value = config
                    .params
                    .get("path")
                    .ok_or(FactoryError::CsvDataFeedWithoutPath)?;
                let path = path_value
                    .clone()
                    .into_string()
                    .map_err(|err| FactoryError::WrongCsvPathFormat(err.to_string()))?;
                Box::new(
                    CsvDataFeed::new(config.name.clone(), path)
                        .map_err(|err| FactoryError::CsvDataFeedInitError(err.to_string()))?,
                )
            }
            DataFeedType::IbMarketDataFeed => {
                let ib_connection = get_ib_connection(Some(&config.params), ib_connections)?;
                let ib_market_data_type = get_param_or_default(
                    &config.params,
                    "data_type",
                    MarketDataType::Delayed,
                    |v: &MarketDataType| Ok(v.clone()),
                    "IB Market Data Feed",
                );
                let feed = IbMarketDataFeed::new(
                    config.name.clone(),
                    ib_connection,
                    config.symbol,
                    ib_market_data_type.into(),
                )
                .map_err(|err| FactoryError::FeedInit(err.to_string()))?;
                Box::new(feed)
            }
            DataFeedType::IbHistoricalDataFeed => {
                let ib_connection: Arc<Client> =
                    get_ib_connection(Some(&config.params), ib_connections)?;

                let default_end_datetime = OffsetDateTime::now_utc();
                let end_datetime = get_param_or_default(
                    &config.params,
                    "end_datetime",
                    default_end_datetime,
                    |s: &String| parse_end_datetime(s).map_err(|e| e.to_string()),
                    "IB Historical Data Feed",
                );

                let default_duration = 7.days();
                let duration = get_param_or_default(
                    &config.params,
                    "duration",
                    default_duration,
                    |s: &String| Duration::try_from(s.as_str()).map_err(|e| e.to_string()),
                    "IB Historical Data Feed",
                );

                let default_bar_size = BarSize::Day;
                let bar_size = get_param_or_default(
                    &config.params,
                    "bar_size",
                    default_bar_size,
                    |s: &String| Ok(BarSize::from(s.as_str())), // TODO: This panics. Consider solving on repo or wrap in Result
                    "IB Historical Data Feed",
                );

                Box::new(IbHistoricalDataFeed::new(
                    config.name.clone(),
                    ib_connection,
                    config.symbol,
                    end_datetime,
                    duration,
                    bar_size,
                ))
            }
        };
        data_feeds.insert(config.name, data_feed);
    }
    Ok(data_feeds)
}

fn build_sizers(
    configs: Vec<PositionSizerConfig>,
) -> Result<HashMap<String, Box<dyn PositionSizer>>, FactoryError> {
    let mut sizers = HashMap::new();
    for config in configs {
        let broker: Box<dyn PositionSizer> = match config.r#type {
            PositionSizerType::FixedSizer => {
                let qty = get_param_or_default(
                    &config.params,
                    "qty",
                    10,
                    |v: &u16| Ok(v.clone() as u32),
                    "Fixed Sizer",
                );
                Box::new(FixedSizer::new(config.name.clone(), qty))
            }
            PositionSizerType::PercentOfEquitySizer => {
                let percent = get_param_or_default(
                    &config.params,
                    "percent",
                    0.1,
                    |v: &f64| Ok(v.clone()),
                    "Percent of Equity Sizer",
                );
                Box::new(PercentOfEquitySizer::new(config.name.clone(), percent))
            }
        };
        sizers.insert(config.name, broker);
    }
    Ok(sizers)
}

fn get_usize_param(params: &Option<HashMap<String, Value>>, key: &str, default: usize) -> usize {
    params
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.clone().into_uint().ok())
        .and_then(|v| v.try_into().ok())
        .unwrap_or(default)
}

fn get_ib_connection(
    params: Option<&HashMap<String, Value>>,
    ib_connections: &HashMap<String, Arc<Client>>,
) -> Result<Arc<Client>, FactoryError> {
    let ib_connection_value = params
        .as_ref()
        .and_then(|p| p.get("connection"))
        .ok_or(FactoryError::IbWithoutConnection)?;
    let ib_connection_name = ib_connection_value
        .clone()
        .into_string()
        .map_err(|err| FactoryError::UnexpectedParameterType(err.to_string()))?;
    let ib_connection = ib_connections
        .get(&ib_connection_name)
        .ok_or(FactoryError::IbConnectionConfigNotFound(ib_connection_name))?;
    Ok(ib_connection.clone())
}

fn get_param_or_default<'de, T, F, V>(
    params: &HashMap<String, Value>,
    key: &str,
    default: T,
    parse: F,
    root_config_name: &str,
) -> T
where
    T: Clone + std::fmt::Debug,
    V: Deserialize<'de> + std::fmt::Debug,
    F: FnOnce(&V) -> Result<T, String>,
{
    match params.get(key) {
        None => {
            warn!(
                "Missing '{key}' param for {root_config_name}. Using default: {:?}",
                default
            );
            default
        }
        Some(v) => match v.clone().try_deserialize() {
            Ok(s) => match parse(&s) {
                Ok(val) => val,
                Err(err) => {
                    error!(
                        "Failed to parse '{key}' param ('{s:?}') for {root_config_name}. \
                        Using default: {:?}. Error: {err}",
                        default
                    );
                    default
                }
            },
            Err(_) => {
                error!(
                    "Wrong type for '{key}' param in {root_config_name}. \
                    Expected string, got something else. Using default: {:?}",
                    default
                );
                default
            }
        },
    }
}

fn parse_end_datetime(s: &str) -> Result<OffsetDateTime, String> {
    if s.eq_ignore_ascii_case("now") {
        return Ok(OffsetDateTime::now_utc());
    }

    // Try "YYYYMMDD HH:MM:SS"
    let fmt_full = format_description!("[year][month][day] [hour]:[minute]:[second]");
    if let Ok(ndt) = PrimitiveDateTime::parse(s, &fmt_full) {
        return Ok(ndt.assume_utc());
    }

    if let Ok(ndt) = PrimitiveDateTime::parse(&(s.to_owned() + " 00:00:00"), &fmt_full) {
        return Ok(ndt.assume_utc());
    }

    Err(format!("Invalid end_datetime format: {s}"))
}

#[derive(Clone, Debug, Deserialize)]
enum MarketDataType {
    Live,
    Frozen,
    Delayed,
    DelayedFrozen,
}

impl From<MarketDataType> for IbMarketDataType {
    fn from(my_type: MarketDataType) -> IbMarketDataType {
        match my_type {
            MarketDataType::Delayed => IbMarketDataType::Delayed,
            MarketDataType::Frozen => IbMarketDataType::Frozen,
            MarketDataType::DelayedFrozen => IbMarketDataType::DelayedFrozen,
            MarketDataType::Live => IbMarketDataType::Live,
        }
    }
}

#[derive(Debug, Error)]
pub enum FactoryError {
    #[error("Interactive Broker configuration without connection parameter")]
    IbWithoutConnection,
    #[error("A different parameter type was expected: `{0}`")]
    UnexpectedParameterType(String),
    #[error("The Interactive Broker connection with name `{0}` was not found in the config")]
    IbConnectionConfigNotFound(String),
    #[error("Failed to create a Interactive Broker connection for `{0}`: `{1}`")]
    IbConnectionFailure(String, String),
    #[error("The broker `{0}` was not found in the conifg")]
    UnknownBroker(String),
    #[error("The data feed `{0}` was not found in the conifg")]
    UnknownDataFeed(String),
    #[error("The position sizer `{0}` was not found in the conifg")]
    UnknownPositionSizer(String),
    #[error("The CSV Data Feed config does not contain a path parameter")]
    CsvDataFeedWithoutPath,
    #[error("The path of the CSV Data Feed config is not the expected format: `{0}`")]
    WrongCsvPathFormat(String),
    #[error("CSV Data Feed initialization failed: `{0}`")]
    CsvDataFeedInitError(String),
    #[error("Failed to initialize broker: `{0}`")]
    BrokerInit(String),
    #[error("Failed to initialize feed: `{0}`")]
    FeedInit(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Value;
    use std::collections::HashMap;
    use time::macros::datetime;

    fn make_params(map: &[(&str, Value)]) -> HashMap<String, Value> {
        map.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_get_param_or_default_returns_default_when_key_missing() {
        let params = HashMap::new();
        let result = get_param_or_default::<u32, _, String>(
            &params,
            "nonexistent",
            42,
            |s| s.parse::<u32>().map_err(|e| e.to_string()),
            "TestConfig",
        );
        assert_eq!(result, 42);
    }

    #[test]
    fn test_get_param_or_default_returns_default_on_wrong_type() {
        let params = make_params(&[("num", 123.5.into())]);
        // parser expects a u32 from a string
        let result = get_param_or_default::<u32, _, String>(
            &params,
            "num",
            10,
            |s| s.parse::<u32>().map_err(|e| e.to_string()),
            "TestConfig",
        );
        assert_eq!(result, 10); // fallback to default
    }

    #[test]
    fn test_get_param_or_default_returns_default_on_parse_error() {
        let params = make_params(&[("num", "not_a_number".into())]);
        let result = get_param_or_default::<u32, _, String>(
            &params,
            "num",
            7,
            |s| s.parse::<u32>().map_err(|e| e.to_string()),
            "TestConfig",
        );
        assert_eq!(result, 7); // fallback
    }

    #[test]
    fn test_get_param_or_default_returns_parsed_value_on_success() {
        let params = make_params(&[("num", "123".into())]);
        let result = get_param_or_default::<u32, _, String>(
            &params,
            "num",
            0,
            |s| s.parse::<u32>().map_err(|e| e.to_string()),
            "TestConfig",
        );
        assert_eq!(result, 123); // parsed successfully
    }

    #[test]
    fn test_get_param_or_default_works_with_non_string_value_type() {
        // Here we try to directly deserialize an integer
        let params = make_params(&[("num", 55.into())]);
        let result = get_param_or_default::<u32, _, u32>(
            &params,
            "num",
            1,
            |n| Ok(n + 5), // parser just adds 5
            "TestConfig",
        );
        assert_eq!(result, 60);
    }

    #[test]
    fn test_parse_end_datetime() {
        // now
        let before = OffsetDateTime::now_utc();
        let parsed = parse_end_datetime("now").unwrap();
        let after = OffsetDateTime::now_utc();

        // parsed should be between before and after
        assert!(parsed >= before && parsed <= after);

        // Full datetime
        assert_eq!(
            parse_end_datetime("20250924 06:04:38").unwrap(),
            datetime!(2025-09-24 6:04:38 +00:00:00)
        );
        // Date only defaults to midnight
        assert_eq!(
            parse_end_datetime("20250924").unwrap(),
            datetime!(2025-09-24 00:00:00 +00:00:00)
        );
        // Invalid input
        assert!(
            parse_end_datetime("invalid-date-string")
                .unwrap_err()
                .contains("Invalid end_datetime format")
        );
    }
}
