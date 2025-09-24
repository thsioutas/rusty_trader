use super::PositionSizer;
use crate::broker::AccountInfo;

pub struct FixedSizer {
    pub name: String,
    qty: u32,
}

impl FixedSizer {
    pub fn new(name: String, qty: u32) -> Self {
        Self { name, qty }
    }
}

impl PositionSizer for FixedSizer {
    fn size(&self, _account: &AccountInfo, _price: f64) -> u32 {
        self.qty
    }
}
