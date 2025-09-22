use super::Broker;
use crate::{broker::BrokerError, types::Order};
use async_trait::async_trait;
use tokio::sync::Mutex;

pub struct DummyBroker {
    name: String,
    orders: Mutex<Vec<Order>>,
}

#[async_trait]
impl Broker for DummyBroker {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError> {
        self.orders.lock().await.push(order.clone());
        Ok(())
    }
}

impl DummyBroker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            orders: Default::default(),
        }
    }

    pub async fn get_orders(&self) -> Vec<Order> {
        self.orders.lock().await.clone()
    }
}
