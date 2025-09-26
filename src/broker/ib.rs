use crate::types::{Fill, Order, OrderSide, Position};
use async_trait::async_trait;
use chrono::Local;
use ibapi::{
    Client,
    accounts::{AccountPortfolioValue, AccountUpdate, AccountValue},
    contracts::Contract,
    orders::{Action, OrderUpdate, order_builder},
};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::info;

use super::{Broker, BrokerError, Portfolio, PortfolioManager};

pub struct Ib {
    name: String,
    client: Arc<Client>,
    portfolio_manager: Arc<PortfolioManager>,
    open_orders: Arc<Mutex<HashMap<i32, Order>>>,
}

#[async_trait]
impl Broker for Ib {
    fn name(&self) -> &str {
        &self.name
    }
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError> {
        let order_id = self
            .client
            .next_valid_order_id()
            .map_err(|err| BrokerError::PlaceOrder(err.to_string()))?;
        // TODO: Do not build only stock contracts
        let contract = Contract::stock(&order.symbol);
        // TODO: Do not build only market orders
        let mut ib_order = order_builder::market_order(order.side.into(), order.qty as f64);
        // TODO: Should this be kept true? Maybe make it configurable?
        ib_order.outside_rth = true;
        let _subscription = self
            .client
            .place_order(order_id, &contract, &ib_order)
            .map_err(|err| BrokerError::PlaceOrder(err.to_string()))?;
        self.open_orders
            .lock()
            .await
            .insert(order_id, order.clone());

        Ok(())
    }
    fn portfolio_manager(&self) -> &PortfolioManager {
        &self.portfolio_manager
    }
}

impl Ib {
    pub fn new(name: String, client: Arc<Client>) -> Result<Self, IbError> {
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
                        cash = value.parse().map_err(|err| {
                            IbError::Init(format!(
                                "Retrieved cash value could not be parsed to float: {}",
                                err
                            ))
                        })?
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
        let portfolio_manager = Arc::new(PortfolioManager::new(portfolio));

        let client_clone = client.clone();
        let open_orders = Arc::new(Mutex::new(HashMap::<i32, Order>::new()));
        let open_orders_clone = open_orders.clone();
        let portfolio_manager_clone = portfolio_manager.clone();
        tokio::spawn(async move {
            // TODO: Consider shutting down this task
            // TODO: When that fails we should get notified and probably stop using that broker
            let updates = client_clone.order_update_stream().unwrap();
            for update in updates {
                match update {
                    OrderUpdate::OrderStatus(status) => {
                        info!(
                            "Interactive Brokers order {} status: {} - filled: {}/{}",
                            status.order_id, status.status, status.filled, status.remaining
                        );

                        if status.status == "Filled" {
                            let order_id = status.order_id;
                            if let Some(order) = open_orders_clone.lock().await.remove(&order_id) {
                                let fill = Fill {
                                    order_id: order_id.to_string(),
                                    symbol: order.symbol,
                                    qty: order.qty,
                                    price: status.average_fill_price,
                                    side: order.side,
                                    timestamp: Local::now().naive_local(),
                                };
                                info!("Apply fill: {:?}", fill);
                                portfolio_manager_clone.apply_fill(fill).await;
                            }
                        }
                    }
                    OrderUpdate::OpenOrder(order_data) => {
                        info!(
                            "Interactive Brokers open order {}: {} {} @ {}",
                            order_data.order.order_id,
                            order_data.order.action,
                            order_data.order.total_quantity,
                            order_data.order.limit_price.unwrap_or(0.0)
                        );
                    }
                    OrderUpdate::Message(notice) => {
                        info!("Interactive Brokers order message: {}", notice.message);
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            name,
            client,
            portfolio_manager,
            open_orders,
        })
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

#[derive(Debug, Error)]
pub enum IbError {
    #[error("Interactive Broker initiazlization failed: {0}")]
    Init(String),
}
