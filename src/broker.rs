use crate::types::{Fill, Order, OrderSide, Position};
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::Mutex;

pub mod dummy;
pub mod ib;

#[async_trait]
pub trait Broker: Send + Sync {
    fn name(&self) -> &str;
    async fn place_order(&self, order: &Order) -> Result<(), BrokerError>;
    fn portfolio_manager(&self) -> &PortfolioManager;
    async fn portfolio_snapshot(&self) -> AccountInfo {
        self.portfolio_manager().snapshot().await
    }
    async fn portfolio_pre_reserve_for_order(
        &self,
        order: &Order,
        current_price: f64,
    ) -> Result<(), PortfolioError> {
        self.portfolio_manager()
            .pre_reserve_for_order(order, current_price)
            .await
    }
    async fn portfolio_release_reserved_cash(&self, qty: u32, price: f64) {
        self.portfolio_manager()
            .release_reserved_cash(qty, price)
            .await
    }
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("Failed to place order: {0}")]
    PlaceOrder(String),
}

pub struct Portfolio {
    pub cash: f64,
    pub reserved_cash: f64,
    pub positions: HashMap<String, Position>,
}

#[derive(Debug)]
pub struct AccountInfo {
    pub cash: f64,          // available cash (liquid)
    pub equity: f64,        // maybe cash + positions marketvalue
    pub reserved_cash: f64, // sums reserved for pending buys
}

impl Portfolio {
    pub fn new(cash: f64, reserved_cash: f64, positions: HashMap<String, Position>) -> Self {
        Self {
            cash,
            reserved_cash,
            positions,
        }
    }

    /// Return read-only snapshot for sizers
    pub fn snapshot(&self) -> AccountInfo {
        let equity = self.cash
            + self
                .positions
                .iter()
                .map(|(_, p)| p.qty as f64 * p.avg_price)
                .sum::<f64>();
        AccountInfo {
            cash: self.cash,
            equity,
            reserved_cash: self.reserved_cash,
        }
    }

    /// Pre-check and reserve funds for a buy order. For sell, check position availability.
    /// This returns Ok if we can proceed (and reserves), or Err if not possible.
    pub fn pre_reserve_for_order(
        &mut self,
        order: &Order,
        current_price: f64,
    ) -> Result<(), PortfolioError> {
        match order.side {
            OrderSide::Buy => {
                let estimated_cost = (order.qty as f64) * current_price;
                if self.cash - self.reserved_cash >= estimated_cost {
                    self.reserved_cash += estimated_cost;
                    Ok(())
                } else {
                    Err(PortfolioError::InsufficientCash(
                        estimated_cost,
                        self.cash - self.reserved_cash,
                    ))
                }
            }
            OrderSide::Sell => {
                let position_qty = self
                    .positions
                    .get(&order.symbol)
                    .map(|p| p.qty)
                    .unwrap_or(0);
                if position_qty >= order.qty {
                    Ok(())
                } else {
                    Err(PortfolioError::InsufficientPosition(
                        order.qty,
                        position_qty,
                    ))
                }
            }
        }
    }

    /// Called once a Fill arrives (from broker). This updates cash, positions, and releases reservations.
    pub fn apply_fill(&mut self, fill: Fill) {
        match fill.side {
            OrderSide::Buy => {
                let cost = fill.price * fill.qty as f64;
                self.reserved_cash -= cost;
                self.cash -= cost;
                let position = self
                    .positions
                    .entry(fill.symbol.clone())
                    .or_insert(Position {
                        symbol: fill.symbol,
                        qty: 0,
                        avg_price: 0.0,
                    });

                // Update avg price of position
                let new_qty = position.qty + fill.qty;
                let prev_value = (position.qty as f64) * position.avg_price;
                let added_value = (fill.qty as f64) * fill.price;
                position.avg_price = (prev_value + added_value) / (new_qty as f64);
                position.qty = new_qty;
            }
            OrderSide::Sell => {
                self.cash += fill.price * fill.qty as f64;
                // TODO: There should always be a position at this point. Consider verifying and return an error
                let position = self.positions.get_mut(&fill.symbol).unwrap();
                position.qty -= fill.qty;
                // Do not update avg_price of position: Because avg_price represents the cost basis of my remaining shares.
                // When I sell, I remove shares at the same cost basis. PnL = (sell_price - avg_price) × qty.
            }
        }
    }

    fn release_reserved_cash(&self, qty: u32, price: f64) {
        // TODO: Implement
        // self.reserved_cash -= (qty as f64) * data.price;
    }
}

#[derive(Debug, Error)]
pub enum PortfolioError {
    #[error("Insufficient cash. Required {0} available {1}")]
    InsufficientCash(f64, f64),
    #[error("Insufficient position. Trying to sell {0} have {1}")]
    InsufficientPosition(u32, u32),
}

pub struct PortfolioManager {
    portfolio: Mutex<Portfolio>,
}

impl PortfolioManager {
    pub fn new(portfolio: Portfolio) -> Self {
        Self {
            portfolio: Mutex::new(portfolio),
        }
    }
    pub async fn snapshot(&self) -> AccountInfo {
        self.portfolio.lock().await.snapshot()
    }
    pub async fn pre_reserve_for_order(
        &self,
        order: &Order,
        current_price: f64,
    ) -> Result<(), PortfolioError> {
        self.portfolio
            .lock()
            .await
            .pre_reserve_for_order(order, current_price)
    }
    async fn release_reserved_cash(&self, qty: u32, price: f64) {
        self.portfolio
            .lock()
            .await
            .release_reserved_cash(qty, price)
    }
    async fn apply_fill(&self, fill: Fill) {
        self.portfolio.lock().await.apply_fill(fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderType;
    fn make_order(symbol: &str, side: OrderSide, qty: u32) -> Order {
        Order {
            symbol: symbol.into(),
            qty,
            price: None,
            side,
            order_type: OrderType::Market,
            strategy_name: Default::default(),
        }
    }

    fn make_fill(symbol: &str, side: OrderSide, qty: u32, price: f64) -> Fill {
        Fill {
            order_id: "test".into(),
            symbol: symbol.into(),
            qty,
            price,
            side,
            timestamp: chrono::Local::now().naive_local(),
        }
    }

    #[test]
    fn test_portfolio_snapshot_includes_positions() {
        let mut positions = HashMap::new();
        positions.insert(
            "AAPL".to_string(),
            Position {
                symbol: "AAPL".into(),
                qty: 10,
                avg_price: 100.0,
            },
        );

        let portfolio = Portfolio::new(500.0, 0.0, positions);
        let snapshot = portfolio.snapshot();

        assert_eq!(snapshot.cash, 500.0);
        assert_eq!(snapshot.equity, 500.0 + 10.0 * 100.0);
        assert_eq!(snapshot.reserved_cash, 0.0);
    }

    #[test]
    fn test_portfolio_pre_reserve_buy_success() {
        let mut portfolio = Portfolio::new(1000.0, 0.0, HashMap::new());
        let order = make_order("AAPL", OrderSide::Buy, 5);

        let result = portfolio.pre_reserve_for_order(&order, 100.0);

        assert!(result.is_ok());
        assert_eq!(portfolio.reserved_cash, 500.0);
    }

    #[test]
    fn test_portfolio_pre_reserve_buy_insufficient_cash() {
        let mut portfolio = Portfolio::new(200.0, 0.0, HashMap::new());
        let order = make_order("AAPL", OrderSide::Buy, 5);

        let result = portfolio.pre_reserve_for_order(&order, 100.0);

        assert!(matches!(result, Err(PortfolioError::InsufficientCash(..))));
    }

    #[test]
    fn tst_portfolio_pre_reserve_sell_success() {
        let mut positions = HashMap::new();
        positions.insert(
            "AAPL".to_string(),
            Position {
                symbol: "AAPL".into(),
                qty: 10,
                avg_price: 100.0,
            },
        );
        let mut portfolio = Portfolio::new(1000.0, 0.0, positions);
        let order = make_order("AAPL", OrderSide::Sell, 5);

        let result = portfolio.pre_reserve_for_order(&order, 150.0);

        assert!(result.is_ok());
    }

    #[test]
    fn test_portfolio_pre_reserve_sell_insufficient_position() {
        let mut positions = HashMap::new();
        positions.insert(
            "AAPL".to_string(),
            Position {
                symbol: "AAPL".into(),
                qty: 2,
                avg_price: 100.0,
            },
        );
        let mut portfolio = Portfolio::new(1000.0, 0.0, positions);
        let order = make_order("AAPL", OrderSide::Sell, 5);

        let result = portfolio.pre_reserve_for_order(&order, 150.0);

        assert!(matches!(
            result,
            Err(PortfolioError::InsufficientPosition(..))
        ));
    }

    #[test]
    fn test_portfolio_apply_fill_buy_updates_cash_and_position() {
        let mut portfolio = Portfolio::new(1000.0, 500.0, HashMap::new());
        let fill = make_fill("AAPL", OrderSide::Buy, 5, 100.0);

        portfolio.apply_fill(fill);

        assert_eq!(portfolio.cash, 1000.0 - 500.0); // spent 500
        assert_eq!(portfolio.reserved_cash, 0.0);

        let pos = portfolio.positions.get("AAPL").unwrap();
        assert_eq!(pos.qty, 5);
        assert_eq!(pos.avg_price, 100.0);
    }

    #[test]
    fn test_portfolio_apply_fill_sell_updates_cash_and_position() {
        let mut positions = HashMap::new();
        positions.insert(
            "AAPL".to_string(),
            Position {
                symbol: "AAPL".into(),
                qty: 10,
                avg_price: 100.0,
            },
        );
        let mut portfolio = Portfolio::new(1000.0, 0.0, positions);
        let fill = make_fill("AAPL", OrderSide::Sell, 5, 150.0);

        portfolio.apply_fill(fill);

        assert_eq!(portfolio.cash, 1000.0 + 750.0);
        let pos = portfolio.positions.get("AAPL").unwrap();
        assert_eq!(pos.qty, 5); // 5 left
        assert_eq!(pos.avg_price, 100.0); // unchanged
    }
}
