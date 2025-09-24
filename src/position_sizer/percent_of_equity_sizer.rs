use super::PositionSizer;
use crate::broker::AccountInfo;

pub struct PercentOfEquitySizer {
    name: String,
    percent: f64,
}

impl PercentOfEquitySizer {
    pub fn new(name: String, percent: f64) -> Self {
        Self { name, percent }
    }
}

impl PositionSizer for PercentOfEquitySizer {
    fn size(&self, account: &AccountInfo, price: f64) -> u32 {
        let max_allocation = account.equity * self.percent;
        let qty = (max_allocation / price).floor();
        qty as u32
    }
}
