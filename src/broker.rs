use crate::types::Order;
use async_trait::async_trait;
use thiserror::Error;

pub mod dummy;
pub mod ib;

#[async_trait]
pub trait Broker: Send + Sync {
    fn name(&self) -> &str;
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError>;
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("Failed to place order")]
    PlaceOrder,
}
