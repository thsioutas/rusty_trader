use crate::types::{Order, OrderSide, Position};
use async_trait::async_trait;
use ibapi::{
    Client, Error as IbError,
    accounts::{AccountPortfolioValue, AccountValue},
    contracts::Contract,
    orders::{Action, order_builder},
    prelude::AccountUpdate,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
};
use tracing::info;

use super::{Broker, BrokerError, Portfolio, PortfolioManager};

pub struct Ib {
    name: String,
    client: Arc<Client>,
    portfolio_manager: PortfolioManager,
    next_order_id: AtomicI32,
}

#[async_trait]
impl Broker for Ib {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError> {
        let order_id = self.next_order_id.fetch_add(1, Ordering::SeqCst);
        // TODO: Do not build only stock contracts
        let contract = Contract::stock(&order.symbol);
        // TODO: Do not build only market orders
        let mut order = order_builder::market_order(order.side.into(), order.qty as f64);
        // TODO: Should this be kept true?
        order.outside_rth = true;
        self.client.place_order(order_id, &contract, &order)?;
        Ok(())
    }
    fn portfolio_manager(&self) -> &PortfolioManager {
        &self.portfolio_manager
    }
}

impl Ib {
    pub fn new(name: String, client: Arc<Client>) -> Self {
        let mut cash = 0.0;
        let reserved_cash = 0.0;
        let mut positions = HashMap::new();
        {
            // TODO: make account configurable
            let subscription = client.account_updates("DUN984059").unwrap();
            for account_update in &subscription {
                match account_update {
                    AccountUpdate::AccountValue(AccountValue {
                        key,
                        value,
                        currency,
                        account,
                    }) if key == "TotalCashValue"
                        && currency == "EUR"
                        && account == Some("DUN984059".to_string()) =>
                    {
                        // TODO: Handle error and do not unwrap
                        cash = value.parse().unwrap()
                    }
                    AccountUpdate::PortfolioValue(AccountPortfolioValue {
                        account,
                        contract,
                        position,
                        average_cost,
                        ..
                    }) if account == Some("DUN984059".to_string()) => {
                        // TODO: Decide what to do with position's currency. The account might be in EUR and the avg_cost here in USD
                        let position = Position {
                            symbol: contract.symbol.clone(),
                            qty: position as u32, // TODO: Consider using f64 for qty of positions
                            avg_price: average_cost,
                        };
                        positions.insert(contract.symbol, position);
                    }
                    AccountUpdate::End => break,
                    _ => {}
                }
            }
        }
        info!(
            "Initialize portfolio for Interactive Brokers ({}) with {} cash, {} \
            reserved cash and the following positions: {:?}",
            name, cash, reserved_cash, positions
        );
        let portfolio = Portfolio::new(cash, 0.0, positions);
        let portfolio_manager = PortfolioManager::new(portfolio);

        let next_order_id = client.next_order_id().into();

        // TODO: Implement a fill listener

        Self {
            name,
            client,
            portfolio_manager,
            next_order_id,
        }
    }
}

impl From<OrderSide> for Action {
    fn from(order_side: OrderSide) -> Self {
        match order_side {
            OrderSide::Buy => Self::Buy,
            OrderSide::Sell => Self::Sell,
        }
    }
}

impl From<IbError> for BrokerError {
    fn from(err: IbError) -> Self {
        Self::PlaceOrder(err.to_string())
    }
}
