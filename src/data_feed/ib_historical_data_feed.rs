use crate::data_feed::{DataFeed, MarketData};
use async_trait::async_trait;
use ibapi::{
    Client,
    contracts::Contract,
    market_data::historical::{BarSize, Duration, WhatToShow},
};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::mpsc;

pub struct IbHistoricalDataFeed {
    name: String,
    rx: mpsc::UnboundedReceiver<MarketData>,
}

impl IbHistoricalDataFeed {
    pub fn new(
        name: String,
        client: Arc<Client>,
        symbol: String,
        interval_end: OffsetDateTime,
        duration: Duration,
        bar_size: BarSize,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let contract = Contract::stock(&symbol);
        let what_to_show = WhatToShow::Trades;
        // Use regular trading hours
        let use_rth = true;
        tokio::spawn(async move {
            // TODO: Handle error by passing it to tx
            let historical_data = client
                .historical_data(
                    &contract,
                    Some(interval_end),
                    duration,
                    bar_size,
                    what_to_show,
                    use_rth,
                )
                .expect("Failed to retrieve data");
            for bar in &historical_data.bars {
                let md = MarketData {
                    symbol: symbol.clone(),
                    price: bar.close,
                };
                let _ = tx.send(md);
            }
        });
        Self { name, rx }
    }
}

#[async_trait]
impl DataFeed for IbHistoricalDataFeed {
    fn name(&self) -> &str {
        &self.name
    }
    async fn next_tick(&mut self) -> Option<MarketData> {
        self.rx.recv().await
    }
}

// #[derive(Debug, Error)]
// pub enum IbHistoricalDataFeedError {
//     #[error("Failed to retrieve historical data: {0}")]
//     HistoricalDataRetrieval(String)
// }
