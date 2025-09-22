use super::Strategy;
use crate::{broker::Broker, data_feed::DataFeed};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

pub struct PrintStrategy {
    pub name: String,
    pub data_feed: Box<dyn DataFeed>,
    pub broker: Arc<dyn Broker>,
}

#[async_trait]
impl Strategy for PrintStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&mut self) {
        info!(
            "Running {} using feed {} on broker {}",
            self.name(),
            self.data_feed.name(),
            self.broker.name()
        );
    }
}
