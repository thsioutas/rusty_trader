use clap::Parser;
use rusty_trader::config::BotConfig;
use rusty_trader::factory::build_strategies;
use tracing::Level;
use tracing_subscriber::fmt;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The log verbosity level
    #[clap(short, long)]
    pub verbosity: Level,
    /// The path to the config file
    #[clap(short, long)]
    pub config: String,
}

#[tokio::main]
async fn main() {
    // Parse CLI args
    let args = Args::parse();

    // Setup logger
    let subscriber = fmt().with_max_level(args.verbosity).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    // Read config file
    let config = BotConfig::deserialize_from_file(&args.config).expect("Failed to read config");

    // Build strategies from config
    let strategies = build_strategies(config)
        .await
        .expect("Failed to build strategies");

    // Fire up strategies
    let mut handles = Vec::new();
    for mut strategy in strategies {
        let handle = tokio::task::spawn(async move { strategy.run().await });
        handles.push(handle);
    }
    futures::future::join_all(handles).await;
    // TODO: Add graceful cleanup
}
