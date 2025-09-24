use super::PositionSizer;

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
    fn size(&self) -> u32 {
        // TODO: Implement
        10
    }
}
