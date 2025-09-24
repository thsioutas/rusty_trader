pub mod fixed_sizer;
pub mod percent_of_equity_sizer;

pub trait PositionSizer: Send + Sync {
    // fn size(&self, account: &AccountInfo, price: f64, signal: &Signal) -> u32;
    fn size(&self) -> u32;
}
