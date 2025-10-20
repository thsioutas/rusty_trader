use async_trait::async_trait;
use ibapi::market_data::MarketDataType;
use ibapi::{Client, contracts::Contract, market_data::realtime::TickTypes};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

use crate::data_feed::{DataFeed, MarketData};

pub struct IbMarketDataFeed {
    name: String,
    rx: mpsc::UnboundedReceiver<MarketData>,
}

impl IbMarketDataFeed {
    pub fn new(
        name: String,
        client: Arc<Client>,
        symbol: String,
        market_data_type: MarketDataType,
    ) -> Result<Self, IbMarketDataFeedError> {
        let (tx, rx) = mpsc::unbounded_channel();
        client
            .switch_market_data_type(market_data_type)
            .map_err(|err| IbMarketDataFeedError::Init(err.to_string()))?;
        let contract = Contract::stock(&symbol);
        // TODO: Fix generic ticks, snapshot and regulatory snapshot
        let generic_ticks = &["233", "293"];
        let snapshot = false;
        let regulatory_snapshot = false;
        tokio::spawn(async move {
            // TODO: Handle error by passing it to tx
            let subscription = client
                .market_data(&contract, generic_ticks, snapshot, regulatory_snapshot)
                .expect("Failed to retireve market data");
            for tick in &subscription {
                match tick {
                    TickTypes::Price(tick_price) => {
                        let md = MarketData {
                            symbol: symbol.clone(),
                            price: tick_price.price,
                            // timestamp:
                        };
                        let _ = tx.send(md);
                    }
                    TickTypes::SnapshotEnd => {
                        subscription.cancel();
                    }
                    _ => {}
                }
            }
        });
        Ok(Self { name, rx })
    }
}

#[async_trait]
impl DataFeed for IbMarketDataFeed {
    fn name(&self) -> &str {
        &self.name
    }
    async fn next_tick(&mut self) -> Option<MarketData> {
        self.rx.recv().await
    }
}

#[derive(Debug, Error)]
pub enum IbMarketDataFeedError {
    #[error("Interactive Broker data feed initialization failed: {0}")]
    Init(String),
}
