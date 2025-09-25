use chrono::NaiveDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub symbol: String,
    pub side: OrderSide,
    pub qty: u32,
    pub price: Option<f64>, // None = market
    pub order_type: OrderType,
    pub strategy_name: String,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub qty: u32,
    pub avg_price: f64,
}

#[derive(Debug)]
pub struct Fill {
    pub order_id: String,
    pub symbol: String,
    pub qty: u32,
    pub price: f64,
    pub side: OrderSide,
    pub timestamp: NaiveDateTime,
}
