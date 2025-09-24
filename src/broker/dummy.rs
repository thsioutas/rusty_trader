use crate::types::{Fill, Order};
use async_trait::async_trait;
use chrono::Local;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::info;

use super::{Broker, BrokerError, Portfolio, PortfolioManager};

pub struct DummyBroker {
    name: String,
    orders: Mutex<Vec<Order>>,
    portfolio_manager: PortfolioManager,
}

#[async_trait]
impl Broker for DummyBroker {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError> {
        self.orders.lock().await.push(order.clone());
        let fill = Fill {
            order_id: Default::default(),
            symbol: order.symbol.clone(),
            qty: order.qty,
            price: order.price.unwrap_or(100.0), // TODO: This should be fixed and use the actual price that it was used
            side: order.side.clone(),
            timestamp: Local::now().naive_local(),
        };
        self.portfolio_manager().apply_fill(fill).await;
        info!(
            "New account status after filled order = {:?}",
            self.portfolio_snapshot().await
        );
        Ok(())
    }
    fn portfolio_manager(&self) -> &PortfolioManager {
        &self.portfolio_manager
    }
}

impl DummyBroker {
    pub fn new(name: String) -> Self {
        let portfolio = Portfolio::new(1000.0, 0.0, HashMap::new());
        let portfolio_manager = PortfolioManager::new(portfolio);
        Self {
            name,
            orders: Default::default(),
            portfolio_manager,
        }
    }

    pub async fn get_orders(&self) -> Vec<Order> {
        self.orders.lock().await.clone()
    }
}
