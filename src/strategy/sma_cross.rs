use crate::{
    broker::{AccountInfo, Broker},
    data_feed::DataFeed,
    position_sizer::PositionSizer,
    strategy::Strategy,
    types::{Order, OrderSide, OrderType},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub const DEFAULT_SMA_CROSS_FAST_WINDOW: usize = 50;
pub const DEFAULT_SMA_CROSS_SLOW_WINDOW: usize = 200;

pub struct SmaCrossStrategy {
    name: String,
    data_feed: Box<dyn DataFeed>,
    broker: Arc<dyn Broker>,
    position_sizer: Box<dyn PositionSizer>,
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
        position_sizer: Box<dyn PositionSizer>,
        fast_window: usize,
        slow_window: usize,
    ) -> Self {
        Self {
            name,
            data_feed,
            broker,
            position_sizer,
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
                let account_snapshot = self.broker.portfolio_snapshot().await;
                let qty = self.position_sizer.size(&account_snapshot, data.price);
                if qty == 0 {
                    info!("Sizer return qty=0; skipping order");
                    continue;
                }
                let order = Order {
                    symbol: data.symbol,
                    side: signal.into(),
                    qty,
                    price: Some(data.price),
                    order_type: OrderType::Market,
                    strategy_name: self.name.clone(),
                };
                match self
                    .broker
                    .portfolio_pre_reserve_for_order(&order, data.price)
                    .await
                {
                    Ok(_) => {
                        match self.broker.place_order(&order).await {
                            Ok(_) => {
                                // TODO: Improve logging
                                info!("Placed {:?} at price {}", order, data.price);
                            }
                            Err(err) => {
                                error!("Failed to place order: {err}");
                                self.broker
                                    .portfolio_release_reserved_cash(qty, data.price)
                                    .await;
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Order pre-check failed: {}", err);
                    }
                }
            } else {
                // TODO: Improve logging. Why no signal at the specific price
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
