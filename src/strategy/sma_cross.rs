use crate::{
    broker::Broker,
    data_feed::DataFeed,
    strategy::Strategy,
    types::{Order, OrderSide, OrderType},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info};

pub const DEFAULT_SMA_CROSS_FAST_WINDOW: usize = 50;
pub const DEFAULT_SMA_CROSS_SLOW_WINDOW: usize = 200;

pub struct SmaCrossStrategy {
    name: String,
    data_feed: Box<dyn DataFeed>,
    broker: Arc<dyn Broker>,
    slow_window: usize,
    fast_window: usize,
    prices: Vec<f64>,
    last_signal: Option<SmaCrossSignal>,
}

impl SmaCrossStrategy {
    pub fn new(
        name: String,
        data_feed: Box<dyn DataFeed>,
        broker: Arc<dyn Broker>,
        fast_window: usize,
        slow_window: usize,
    ) -> Self {
        Self {
            name,
            data_feed,
            broker,
            fast_window,
            slow_window,
            prices: Vec::new(),
            last_signal: None,
        }
    }

    fn sma(&self, window: usize) -> Option<f64> {
        if self.prices.len() < window {
            return None;
        }
        let slice = &self.prices[self.prices.len() - window..];
        Some(slice.iter().copied().sum::<f64>() / window as f64)
    }

    fn check_signal(&mut self) -> Option<SmaCrossSignal> {
        let fast = self.sma(self.fast_window)?;
        let slow = self.sma(self.slow_window)?;

        let new_signal = if fast > slow {
            Some(SmaCrossSignal::Buy)
        } else if fast < slow {
            Some(SmaCrossSignal::Sell)
        } else {
            None
        };
        if new_signal != self.last_signal {
            self.last_signal = new_signal;
            return new_signal;
        }
        None
    }
}

#[async_trait]
impl Strategy for SmaCrossStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&mut self) {
        while let Some(data) = self.data_feed.next_tick().await {
            self.prices.push(data.price);
            if let Some(signal) = self.check_signal() {
                // TODO: Fix size of order
                let order = Order {
                    symbol: "AAPL".into(),
                    side: signal.into(),
                    qty: 10,
                    price: None,
                    order_type: OrderType::Market,
                };
                if let Err(err) = self.broker.place_order(&order).await {
                    error!("Failed to place order: {err}");
                } else {
                    info!("Placed {:?} at price {}", order, data.price);
                }
            } else {
                debug!("No signal at price {}", data.price);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmaCrossSignal {
    Buy,
    Sell,
}

impl From<SmaCrossSignal> for OrderSide {
    fn from(signal: SmaCrossSignal) -> Self {
        match signal {
            SmaCrossSignal::Buy => Self::Buy,
            SmaCrossSignal::Sell => Self::Sell,
        }
    }
}
