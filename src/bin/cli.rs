use anyhow::Result;
use clap::{Parser, Subcommand};
use vnquant_dataset::finance::{db::Database, utils::fetch_tickers};

#[derive(Parser)]
#[command(name = "vnquant")]
#[command(about = "A CLI tool for managing financial data")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
