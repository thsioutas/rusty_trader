use async_trait::async_trait;

pub mod csv_data_feed;
pub mod ib_market_data_feed;

#[derive(Debug, Clone)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    // pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait DataFeed: Send + Sync {
    fn name(&self) -> &str;
    async fn next_tick(&mut self) -> Option<MarketData>;
}
