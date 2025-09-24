use rusty_trader::broker::dummy::DummyBroker;
use rusty_trader::data_feed::csv_data_feed::CsvDataFeed;
use rusty_trader::position_sizer::fixed_sizer::FixedSizer;
use rusty_trader::strategy::Strategy;
use rusty_trader::strategy::sma_cross::SmaCrossStrategy;
use rusty_trader::types::OrderSide;
use std::sync::Arc;

mod common;
use common::generate_backtest_csv;

#[tokio::test]
async fn test_sma_cross_strategy_signals() {
    let csv_feed_file = generate_backtest_csv();
    let path = csv_feed_file.path().to_string_lossy().to_string();

    let feed = CsvDataFeed::new("backtest".to_string(), path).unwrap();
    let broker = Arc::new(DummyBroker::new("Dummy".to_string()));
    let mut strat = SmaCrossStrategy::new(
        "TestSMA".to_string(),
        Box::new(feed),
        broker.clone(),
        Box::new(FixedSizer::new("Fixed sizer".into(), 100)),
        50,
        200,
    );
    strat.run().await;
    let orders = broker.get_orders().await;
    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].side, OrderSide::Buy);
    assert_eq!(orders[1].side, OrderSide::Sell);
}
