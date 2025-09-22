#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub price: Option<f64>,
    pub order_type: OrderType,
    // pub timestamp: chrono::NaiveDateTime,
    // pub strategy_name: String,
}
