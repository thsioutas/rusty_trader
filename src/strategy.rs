use async_trait::async_trait;

pub mod print;
pub mod sma_cross;

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&mut self);
}
