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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::AccountInfo;

    #[test]
    fn test_percent_of_equity_sizer_size() {
        let sizer = PercentOfEquitySizer::new("equity-sizer".to_string(), 0.1);
        let account = AccountInfo {
            cash: 0.0,
            reserved_cash: 0.0,
            equity: 1000.0,
        };
        assert_eq!(1, sizer.size(&account, 100.0));
    }
}
