use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tradingview::{Interval, UserCookies, get_quote_token};
use vnquant_dataset::finance::{
    db::Database,
    models::Ticker,
    utils::{fetch_intraday_prices_all, fetch_prices, fetch_prices_all, fetch_tickers},
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
    GetToken {
        #[arg(env = "TV_COOKIES")]
        cookies: Option<String>,

        #[arg(short, long)]
        cookies_path: Option<String>,
    },
    LoginTradingview {
        /// Username for TradingView
        #[arg(short, long, env = "TV_USERNAME")]
        username: String,

        /// Password for TradingView
        #[arg(short, long, env = "TV_PASSWORD")]
        password: String,

        // optional TradingView token for 2FA
        #[arg(short, long, env = "TV_TOTP_SECRET")]
        totp_secret: Option<String>,

        // path to save cookies
        #[arg(short, long, default_value = "cookies.json")]
        cookies_path: String,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
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
    /// Fetch intraday prices for all tickers in the database
    FetchIntradayPricesAll {
        /// Database URL (can also be set via DATABASE_URL environment variable)
        #[arg(long, env = "DATABASE_URL")]
        database_url: String,

        /// Time interval for price data
        #[arg(short, long, value_enum, default_value = "one-hour")]
        interval: IntervalArg,

        /// Number of concurrent requests
        #[arg(short, long, default_value = "5")]
        concurrency: usize,

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

            fetch_prices_all(db, interval.into(), 100, 2).await?;

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

        Commands::FetchIntradayPricesAll {
            database_url,
            interval,
            concurrency,
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
                "📊 Fetching intraday prices for all tickers with interval {:?} (concurrency: {})...",
                interval, concurrency
            );
            let start = std::time::Instant::now();

            fetch_intraday_prices_all(&db, interval.into(), concurrency).await?;

            let duration = start.elapsed();
            println!(
                "✅ Successfully fetched intraday prices for all tickers in {:.2}s!",
                duration.as_secs_f64()
            );
        }
        Commands::LoginTradingview {
            username,
            password,
            totp_secret,
            verbose,
            cookies_path,
        } => {
            // Initialize logging
            let log_level = if verbose {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            };

            tracing_subscriber::fmt().with_max_level(log_level).init();

            let user = UserCookies::default()
                .login(&username, &password, totp_secret.as_deref())
                .await?;

            // save cookies to file
            serde_json::to_writer(std::fs::File::create(&cookies_path)?, &user)?;

            if verbose {
                println!("🔐 Successfully logged in to TradingView as {}", username);
                println!(
                    "Cookies saved to {}",
                    std::fs::canonicalize(&cookies_path)?.display()
                );
            } else {
                println!("🔐 Login successful. Cookies saved.");
            }
        }
        Commands::GetToken {
            cookies,
            cookies_path,
        } => {
            // Load cookies from file or environment variable
            let cookies = if let Some(path) = cookies_path {
                std::fs::read_to_string(path)?
            } else {
                cookies.ok_or_else(|| {
                    anyhow::anyhow!("No cookies provided. Please set TV_COOKIES environment variable or use --cookies-path option.")
                })?
            };

            // Parse cookies JSON
            let user: UserCookies = serde_json::from_str(&cookies)?;

            let token = get_quote_token(&user).await?;

            // Print the auth token
            println!("{}", token);

            // set TV_AUTH_TOKEN environment variable
            unsafe {
                std::env::set_var("TV_AUTH_TOKEN", &token);
            }
        }
    }

    Ok(())
}
