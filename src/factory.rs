use crate::{
    broker::{Broker, dummy::DummyBroker, ib::Ib},
    config::{
        BotConfig, BrokerConfig, BrokerType, DataFeedConfig, DataFeedType, IbConnectionConfig,
        StrategyType,
    },
    data_feed::{
        DataFeed, csv_data_feed::CsvDataFeed, ib_historical_data_feed::IbHistoricalDataFeed,
        ib_market_data_feed::IbMarketDataFeed,
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
    market_data::historical::{BarSize, Duration, ToDuration},
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use time::{OffsetDateTime, PrimitiveDateTime, macros::format_description};
use tracing::{debug, error, warn};

pub async fn build_strategies(
    bot_config: BotConfig,
) -> Result<Vec<Box<dyn Strategy>>, FactoryError> {
    let ib_connections = build_connections(bot_config.ib_connections)?;
    let brokers = build_brokers(bot_config.brokers, &ib_connections)?;
    let mut data_feeds = build_data_feeds(bot_config.data_feeds, &ib_connections)?;
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
        match config.r#type {
            StrategyType::PrintStrategy => {
                let strategy: Box<dyn Strategy> = Box::new(PrintStrategy {
                    name: config.name,
                    data_feed,
                    broker: broker.clone(),
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
                Arc::new(Ib::new(config.name.clone(), ib_connection.clone()))
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
                Box::new(IbMarketDataFeed::new(config.name.clone(), ib_connection))
            }
            DataFeedType::IbHistoricalDataFeed => {
                let ib_connection: Arc<Client> =
                    get_ib_connection(Some(&config.params), ib_connections)?;

                let default_end_datetime = OffsetDateTime::now_utc();
                let end_datetime = get_ib_historical_data_feed_param_or_default(
                    &config.params,
                    "end_datetime",
                    default_end_datetime,
                    |s| parse_end_datetime(s).map_err(|e| e.to_string()),
                );

                let default_duration = 7.days();
                let duration = get_ib_historical_data_feed_param_or_default(
                    &config.params,
                    "duration",
                    default_duration,
                    |s| Duration::try_from(s).map_err(|e| e.to_string()),
                );

                let default_bar_size = BarSize::Day;
                let bar_size = get_ib_historical_data_feed_param_or_default(
                    &config.params,
                    "bar_size",
                    default_bar_size,
                    |s| Ok(BarSize::from(s)), // TODO: This panics. Consider solving on repo or wrap in Result
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

fn get_ib_historical_data_feed_param_or_default<T, F>(
    params: &HashMap<String, Value>,
    key: &str,
    default: T,
    parse: F,
) -> T
where
    T: Clone + std::fmt::Debug,
    F: FnOnce(&str) -> Result<T, String>,
{
    match params.get(key) {
        None => {
            warn!(
                "Missing '{key}' param for IB Historical Data Feed. Using default: {:?}",
                default
            );
            default
        }
        Some(v) => match v.clone().into_string() {
            Ok(s) => match parse(&s) {
                Ok(val) => val,
                Err(err) => {
                    error!(
                        "Failed to parse '{key}' param ('{s}') for IB Historical Data Feed. \
                        Using default: {:?}. Error: {err}",
                        default
                    );
                    default
                }
            },
            Err(_) => {
                error!(
                    "Wrong type for '{key}' param in IB Historical Data Feed. \
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
    #[error("The CSV Data Feed config does not contain a path parameter")]
    CsvDataFeedWithoutPath,
    #[error("The path of the CSV Data Feed config is not the expected format: `{0}`")]
    WrongCsvPathFormat(String),
    #[error("CSV Data Feed initialization failed: `{0}`")]
    CsvDataFeedInitError(String),
}
