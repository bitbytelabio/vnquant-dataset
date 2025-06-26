use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tradingview::Interval;
use vnquant_dataset::finance::{
    db::Database,
    models::Ticker,
    utils::{fetch_prices, fetch_prices_all_tickers_chunked_with_retry, fetch_tickers},
};

#[derive(Parser)]
#[command(name = "vnquant")]
#[command(about = "A CLI tool for managing financial data")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum, Debug)]
enum IntervalArg {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    TwoHours,
    FourHours,
    OneDay,
    OneWeek,
    OneMonth,
}

impl From<IntervalArg> for Interval {
    fn from(interval: IntervalArg) -> Self {
        match interval {
            IntervalArg::OneMinute => Interval::OneMinute,
            IntervalArg::FiveMinutes => Interval::FiveMinutes,
            IntervalArg::FifteenMinutes => Interval::FifteenMinutes,
            IntervalArg::ThirtyMinutes => Interval::ThirtyMinutes,
            IntervalArg::OneHour => Interval::OneHour,
            IntervalArg::TwoHours => Interval::TwoHours,
            IntervalArg::FourHours => Interval::FourHours,
            IntervalArg::OneDay => Interval::OneDay,
            IntervalArg::OneWeek => Interval::OneWeek,
            IntervalArg::OneMonth => Interval::OneMonth,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch tickers from TradingView exchanges
    FetchTickers {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Fetch prices for all tickers in the database
    FetchPricesAll {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Time interval for price data
        #[arg(short, long, value_enum, default_value = "one-day")]
        interval: IntervalArg,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Fetch prices for a specific ticker
    FetchPrices {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Ticker symbol
        #[arg(short, long)]
        symbol: String,

        /// Exchange name
        #[arg(short, long)]
        exchange: String,

        /// Time interval for price data
        #[arg(short, long, value_enum, default_value = "one-day")]
        interval: IntervalArg,

        /// Enable replay mode
        #[arg(short, long)]
        replay: bool,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// List all tickers in the database
    ListTickers {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Filter by exchange
        #[arg(short, long)]
        exchange: Option<String>,

        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get information about a specific ticker
    GetTicker {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Ticker symbol
        #[arg(short, long)]
        symbol: String,

        /// Exchange name
        #[arg(short, long)]
        exchange: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::FetchTickers {
            database_url,
            verbose,
        } => {
            // Initialize logging
            let log_level = if verbose {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            };

            tracing_subscriber::fmt().with_max_level(log_level).init();

            println!("🔄 Connecting to database...");
            let db = Database::new(&database_url).await?;

            println!("📈 Fetching tickers from exchanges...");
            fetch_tickers(db).await?;

            println!("✅ Successfully fetched and stored tickers!");
        }

        Commands::FetchPricesAll {
            database_url,
            interval,
            verbose,
        } => {
            // Initialize logging
            let log_level = if verbose {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            };

            tracing_subscriber::fmt().with_max_level(log_level).init();

            println!("🔄 Connecting to database...");
            let db = Database::new(&database_url).await?;

            println!(
                "📊 Fetching prices for all tickers with interval {:?}...",
                interval
            );
            let start = std::time::Instant::now();

            fetch_prices_all_tickers_chunked_with_retry(db, interval.into(), 100, 2).await?;

            let duration = start.elapsed();
            println!(
                "✅ Successfully fetched prices for all tickers in {:.2}s!",
                duration.as_secs_f64()
            );
        }

        Commands::FetchPrices {
            database_url,
            symbol,
            exchange,
            interval,
            replay,
            verbose,
        } => {
            // Initialize logging
            let log_level = if verbose {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            };

            tracing_subscriber::fmt().with_max_level(log_level).init();

            println!("🔄 Connecting to database...");
            let db = Database::new(&database_url).await?;

            let ticker = Ticker::builder()
                .symbol(symbol.clone())
                .exchange(exchange.clone())
                .build();

            println!(
                "📊 Fetching prices for {}:{} with interval {:?}...",
                symbol, exchange, interval
            );
            let start = std::time::Instant::now();

            fetch_prices(db, &ticker, interval.into(), replay).await?;

            let duration = start.elapsed();
            println!(
                "✅ Successfully fetched prices for {}:{} in {:.2}s!",
                symbol,
                exchange,
                duration.as_secs_f64()
            );
        }

        Commands::ListTickers {
            database_url,
            exchange,
            limit,
        } => {
            let db = Database::new(&database_url).await?;

            let tickers = if let Some(exchange_name) = exchange {
                db.get_tickers_by_exchange(&exchange_name).await?
            } else {
                db.get_all_tickers().await?
            };

            let display_tickers = if let Some(limit_count) = limit {
                tickers.into_iter().take(limit_count).collect::<Vec<_>>()
            } else {
                tickers
            };

            if display_tickers.is_empty() {
                println!("No tickers found.");
            } else {
                println!("Found {} tickers:", display_tickers.len());
                println!(
                    "{:<15} {:<15} {:<30} {:<10}",
                    "Symbol", "Exchange", "Description", "Currency"
                );
                println!("{}", "-".repeat(70));

                for ticker in display_tickers {
                    println!(
                        "{:<15} {:<15} {:<30} {:<10}",
                        ticker.symbol,
                        ticker.exchange,
                        ticker.description.as_deref().unwrap_or("N/A"),
                        ticker.currency.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }

        Commands::GetTicker {
            database_url,
            symbol,
            exchange,
        } => {
            let db = Database::new(&database_url).await?;

            match db.get_ticker(&symbol, &exchange).await? {
                Some(ticker) => {
                    println!("Ticker Information:");
                    println!("Symbol: {}", ticker.symbol);
                    println!("Exchange: {}", ticker.exchange);
                    println!(
                        "Description: {}",
                        ticker.description.as_deref().unwrap_or("N/A")
                    );
                    println!("Currency: {}", ticker.currency.as_deref().unwrap_or("N/A"));
                    println!("Country: {}", ticker.country.as_deref().unwrap_or("N/A"));
                    println!(
                        "Market Type: {}",
                        ticker.market_type.as_deref().unwrap_or("N/A")
                    );
                    println!("Industry: {}", ticker.industry.as_deref().unwrap_or("N/A"));
                    println!("Sector: {}", ticker.sector.as_deref().unwrap_or("N/A"));
                    if let Some(founded) = ticker.founded {
                        println!("Founded: {}", founded);
                    }
                }
                None => {
                    println!("Ticker '{}' not found on exchange '{}'", symbol, exchange);
                }
            }
        }
    }

    Ok(())
}
