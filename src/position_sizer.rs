use crate::broker::AccountInfo;

pub mod fixed_sizer;
pub mod percent_of_equity_sizer;

// TODO: Consider combining PositionSizer with PortfolioManager
pub trait PositionSizer: Send + Sync {
    fn size(&self, account: &AccountInfo, price: f64) -> u32;
}
