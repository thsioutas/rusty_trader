# rusty_trader

`rusty_trader` is a modular trading bot framework written in Rust.  
It is designed for backtesting and live trading, supporting multiple **data feeds**, **brokers**, **sizers** and **strategies**.

## Features
- **Pluggable brokers**: Interactive Brokers (IB) and a Dummy broker for testing.
- **Pluggable data feeds**: CSV backtesting, IB market data, IB historical data.
- **Pluggable sizers**: Fixed, percent of equity, percent of available cash.
- **Multiple strategies** per config file.
- **Strategy-specific parameters** (e.g., SMA fast/slow windows).
- **Shared IB connections** across brokers and data feeds.
- **Async execution** with `tokio`.


## System design
The application is built around **four main abstractions**:

- **Brokers** → Responsible for placing orders (e.g. Interactive Brokers, DummyBroker).
- **Data feeds** → Provide data streams (live, historical, or CSV for backtesting).
- **Strategies** → Contain trading logic and consume a broker + their own dedicated data feed.
- **Sizers** → Control how many units (shares/contracts) each strategy buys or sells.

### Relationships

- Multiple **brokers** can be defined in the config.
- **Strategies share brokers** (e.g. all use the same IB broker).
- Each **strategy has its own data feed** (no central feed).
- Each **strategy has its own sizer**.
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
  - name: "ib-local"
    address: "127.0.0.1:4040"
    client_id: 1

brokers:
  - name: "ib-broker"
    type: "IbBroker"
    params:
      connection: "ib-local"

data_feeds:
  - name: "ib-market-data-feed"
    type: "IbMarketDataFeed"
    params:
      connection: "ib-local"

sizers:
  - name: "fixed-100"
    type: "FixedSizer"
    params:
      qty: 100

strategies:
  - name: "SMA cross with market data"
    type: "SmaCrossStrategy"
    broker: "ib-broker"
    data_feed: "ib-market-data-feed"
    sizer: "fixed-100"
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
