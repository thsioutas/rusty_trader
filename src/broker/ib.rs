use async_trait::async_trait;
use ibapi::Client;
use std::sync::Arc;

use crate::{broker::BrokerError, types::Order};

use super::Broker;

pub struct Ib {
    name: String,
    client: Arc<Client>,
}

#[async_trait]
impl Broker for Ib {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, _order: &Order) -> Result<(), BrokerError> {
        Ok(())
    }
}

impl Ib {
    pub fn new(name: String, client: Arc<Client>) -> Self {
        Self { name, client }
    }
}
