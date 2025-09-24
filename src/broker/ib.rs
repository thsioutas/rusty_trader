use crate::types::Order;
use async_trait::async_trait;
use ibapi::{
    Client,
    accounts::{AccountSummaries, AccountSummaryTags},
};
use std::{collections::HashMap, sync::Arc};

use super::{Broker, BrokerError, Portfolio, PortfolioManager};

pub struct Ib {
    name: String,
    client: Arc<Client>,
    portfolio_manager: PortfolioManager,
}

#[async_trait]
impl Broker for Ib {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError> {
        Ok(())
    }
    fn portfolio_manager(&self) -> &PortfolioManager {
        &self.portfolio_manager
    }
}

impl Ib {
    pub fn new(name: String, client: Arc<Client>) -> Self {
        let cash = {
            // TODO: Find a way to only get a specific account
            let group = "All";
            // TODO: Get reserved cash and available positions (name, qty and average cost);
            let tags = &[AccountSummaryTags::TOTAL_CASH_VALUE];
            let subscription = client
                .account_summary(group, tags)
                .expect("error requesting account summary");
            let account_summary = subscription.iter().next().unwrap();
            // TODO: Do not panic here. Return error and handle accordingly
            match account_summary {
                AccountSummaries::Summary(summary) => summary.value.parse().unwrap(),
                _ => panic!("Should get a summary"),
            }
        };
        let portfolio = Portfolio::new(cash, 0.0, HashMap::new());
        let portfolio_manager = PortfolioManager::new(portfolio);
        // TODO: Implement a fill listener

        Self {
            name,
            client,
            portfolio_manager,
        }
    }
}
