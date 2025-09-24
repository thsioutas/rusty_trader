use super::PositionSizer;

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
    fn size(&self) -> u32 {
        // TODO: Check account info etc
        self.qty
    }
}
