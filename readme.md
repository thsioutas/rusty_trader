# rusty_trader

`rusty_trader` is a modular trading bot framework written in Rust.  
It is designed for backtesting and live trading, supporting multiple **data feeds**, **strategies** and **brokers**.

## Features
- **Pluggable brokers**: Interactive Brokers (IB) and a Dummy broker for testing.
- **Pluggable data feeds**: CSV backtesting, IB market data, IB historical data.
- **Multiple strategies** per config file.
- **Strategy-specific parameters** (e.g., SMA fast/slow windows).
- **Shared IB connections** across brokers and data feeds.
- **Async execution** with `tokio`.


## System design
The application is built around **three main abstractions**:

- **Brokers** → Responsible for placing orders (e.g. Interactive Brokers, DummyBroker).
- **Data feeds** → Provide data streams (live, historical, or CSV for backtesting).
- **Strategies** → Contain trading logic and consume a broker + their own dedicated data feed.

### Relationships

- Multiple **brokers** can be defined in the config.
- **Strategies share brokers** (e.g. all use the same IB broker).
- Each **strategy has its own data feed** (no central feed).
- Interactive Brokers **connections** (host/port/client_id) can be **shared** between:
  - multiple brokers,
  - and multiple data feeds.

<!-- ### Diagram

```text
                ┌─────────────────────────────────────────┐
                │   IB Connection (host:port, client_id)  │
                └─────────────┬───────────────────────────┘ 
                              │
        ┌─────────────────────┼───────────────────────┐
        │                     │                       │
┌───────▼────────┐   ┌────────▼─────────┐    ┌────────▼─────────┐
│   IB Broker    │   │ IB Market Feed   │    │ IB Historical    │
│ (places orders)│   │ (live prices)    │    │ Data Feed        │
└───────┬────────┘   └────────┬─────────┘    └────────┬─────────┘
        │                     │                       │
        │                     │                       │
   ┌────▼────┐            ┌───▼──────┐            ┌───▼──────┐
   │Strategy │            │Strategy  │            │Strategy  │
   │ SMA(200)│            │ SMA(200) │            │ Print    │
   └─────────┘            └──────────┘            └──────────┘
``` -->


## Command-line usage
```bash
cargo run -- --config config.yaml --verbosity info
```
Arguments:

* --config, -c: Path to the YAML configuration file.

* --verbosity, -v: Log verbosity level (error, warn, info, debug, trace).

## Example config

```yaml
ib_connections:
  - name: "IB-local"
    address: "127.0.0.1:4040"
    client_id: 1

brokers:
  - name: "IB broker"
    type: "IbBroker"
    params:
      connection: "IB-local"

data_feeds:
  - name: "IB Market data feed"
    type: "IbMarketDataFeed"
    params:
      connection: "IB-local"

strategies:
  - name: "SMA cross with market data"
    type: "SmaCrossStrategy"
    broker: "IB broker"
    data_feed: "IB Market data feed"
    params:
      slow_window: 200
      fast_window: 50
```

A full example is provided here:
[`example_configs/config.yaml`](example_configs/config.yaml)

## Running tests
```bash
cargo test
```
