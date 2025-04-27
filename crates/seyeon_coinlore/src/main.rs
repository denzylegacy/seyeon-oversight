use seyeon_coinlore::CoinloreClient;
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let client = CoinloreClient::new();
    let command = &args[1];

    match command.as_str() {
        "global" => {
            println!("Fetching global cryptocurrency market data...\n");
            
            match client.get_global_market_data().await {
                Ok(data) => {
                    if let Some(market_data) = data.first() {
                        println!("Global Crypto Market Overview\n");
                        println!("Total cryptocurrencies: {}", market_data.coins_count);
                        println!("Active markets: {}", market_data.active_markets);
                        println!("Total market cap: ${:.2}", market_data.total_mcap);
                        println!("Total 24h volume: ${:.2}", market_data.total_volume);
                        println!("Bitcoin dominance: {}%", market_data.btc_d);
                        println!("Ethereum dominance: {}%", market_data.eth_d);
                        println!("Market cap change (24h): {}%", market_data.mcap_change);
                        println!("Volume change (24h): {}%", market_data.volume_change);
                        println!("Average price change (24h): {}%", market_data.avg_change_percent);
                    } else {
                        println!("No global market data available");
                    }
                },
                Err(err) => eprintln!("Error fetching global market data: {}", err),
            }
        },
        
        "tickers" => {
            let start = args.get(2).map(|s| s.parse::<u32>().unwrap_or(0)).unwrap_or(0);
            let limit = args.get(3).map(|s| s.parse::<u32>().unwrap_or(10)).unwrap_or(10);
            
            println!("Fetching tickers (start: {}, limit: {})...\n", start, limit);
            
            match client.get_tickers(Some(start), Some(limit)).await {
                Ok(tickers) => {
                    println!("Cryptocurrency Tickers\n");
                    println!("Total coins: {}", tickers.info.coins_num);
                    println!("Timestamp: {}", tickers.info.time);
                    println!("\nDisplaying {} coins:\n", tickers.data.len());
                    
                    for ticker in tickers.data {
                        println!("#{} - {} ({})", ticker.rank, ticker.name, ticker.symbol);
                        println!("  Price: ${}", ticker.price_usd);
                        println!("  Market Cap: ${}", ticker.market_cap_usd);
                        println!("  24h Change: {}%", ticker.percent_change_24h);
                        println!("  24h Volume: ${:.2}", ticker.volume24);
                        println!();
                    }
                },
                Err(err) => eprintln!("Error fetching tickers: {}", err),
            }
        },
        
        "ticker" => {
            if args.len() < 3 {
                eprintln!("Please provide at least one coin ID");
                process::exit(1);
            }
            
            let ids: Vec<&str> = args[2].split(',').collect();
            println!("Fetching data for coin IDs: {}...\n", args[2]);
            
            match client.get_ticker(&ids).await {
                Ok(tickers) => {
                    println!("Specific Coin Data\n");
                    
                    for ticker in tickers {
                        println!("#{} - {} ({})", ticker.rank, ticker.name, ticker.symbol);
                        println!("  ID: {}", ticker.id);
                        println!("  Price: ${}", ticker.price_usd);
                        println!("  Market Cap: ${}", ticker.market_cap_usd);
                        println!("  1h Change: {}%", ticker.percent_change_1h);
                        println!("  24h Change: {}%", ticker.percent_change_24h);
                        println!("  7d Change: {}%", ticker.percent_change_7d);
                        println!("  Circulating Supply: {}", ticker.csupply);
                        if let Some(tsupply) = &ticker.tsupply {
                            println!("  Total Supply: {}", tsupply);
                        }
                        if let Some(msupply) = &ticker.msupply {
                            println!("  Max Supply: {}", msupply);
                        }
                        println!();
                    }
                },
                Err(err) => eprintln!("Error fetching ticker data: {}", err),
            }
        },
        
        "markets" => {
            if args.len() < 3 {
                eprintln!("Please provide a coin ID");
                process::exit(1);
            }
            
            let coin_id = &args[2];
            println!("Fetching markets for coin ID: {}...\n", coin_id);
            
            match client.get_coin_markets(coin_id).await {
                Ok(markets) => {
                    println!("Markets for Coin ID: {}\n", coin_id);
                    println!("Total markets: {}\n", markets.len());
                    
                    for (i, market) in markets.iter().enumerate() {
                        println!("{}. {} - {}/{}", i+1, market.name, market.base, market.quote);
                        println!("   Price: ${:.8}", market.price_usd);
                        println!("   Volume: ${:.2}", market.volume_usd);
                        println!("   Last Updated: {}", market.time);
                        println!();
                    }
                },
                Err(err) => eprintln!("Error fetching coin markets: {}", err),
            }
        },
        
        "exchanges" => {
            println!("Fetching all exchanges...\n");
            
            match client.get_exchanges().await {
                Ok(exchanges) => {
                    let exchanges_vec: Vec<_> = exchanges.values().collect();
                    println!("Cryptocurrency Exchanges\n");
                    println!("Total exchanges: {}\n", exchanges_vec.len());
                    
                    for (i, exchange) in exchanges_vec.iter().enumerate() {
                        println!("{}. {} (ID: {})", i+1, exchange.name, exchange.id);
                        println!("   Country: {}", exchange.country);
                        println!("   Active Pairs: {}", exchange.active_pairs);
                        println!("   Volume (USD): ${:.2}", exchange.volume_usd);
                        println!("   URL: {}", exchange.url);
                        println!();
                    }
                },
                Err(err) => eprintln!("Error fetching exchanges: {}", err),
            }
        },
        
        "exchange" => {
            if args.len() < 3 {
                eprintln!("Please provide an exchange ID");
                process::exit(1);
            }
            
            let exchange_id = &args[2];
            println!("Fetching data for exchange ID: {}...\n", exchange_id);
            
            match client.get_exchange(exchange_id).await {
                Ok(exchange) => {
                    println!("Exchange: {}\n", exchange.info.name);
                    println!("URL: {}", exchange.info.url);
                    println!("Date Live: {}", exchange.info.date_live);
                    println!("Total pairs: {}\n", exchange.pairs.len());
                    
                    println!("Top Trading Pairs:");
                    for (i, pair) in exchange.pairs.iter().take(10).enumerate() {
                        println!("{}. {}/{}", i+1, pair.base, pair.quote);
                        println!("   Price: ${:.8}", pair.price_usd);
                        println!("   Volume: ${:.2}", pair.volume);
                        println!();
                    }
                    
                    if exchange.pairs.len() > 10 {
                        println!("... and {} more pairs", exchange.pairs.len() - 10);
                    }
                },
                Err(err) => eprintln!("Error fetching exchange data: {}", err),
            }
        },
        
        "social" => {
            if args.len() < 3 {
                eprintln!("Please provide a coin ID");
                process::exit(1);
            }
            
            let coin_id = &args[2];
            println!("Fetching social stats for coin ID: {}...\n", coin_id);
            
            match client.get_social_stats(coin_id).await {
                Ok(stats) => {
                    println!("Social Media Stats\n");
                    
                    println!("Reddit:");
                    println!("  Subscribers: {}", stats.reddit.subscribers);
                    println!("  Avg. Active Users: {:.2}", stats.reddit.avg_active_users);
                    
                    println!("\nTwitter:");
                    println!("  Followers: {}", stats.twitter.followers_count);
                    println!("  Tweets: {}", stats.twitter.status_count);
                },
                Err(err) => eprintln!("Error fetching social stats: {}", err),
            }
        },
        
        _ => {
            println!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Coinlore API Client\n");
    println!("USAGE:");
    println!("  seyeon_coinlore <COMMAND> [ARGS...]\n");
    println!("COMMANDS:");
    println!("  global                    Get global cryptocurrency market data");
    println!("  tickers [start] [limit]   Get tickers for multiple coins (default: start=0, limit=10)");
    println!("  ticker <id1,id2,...>      Get data for specific coin(s) by ID(s)");
    println!("  markets <coin_id>         Get markets for a specific coin by ID");
    println!("  exchanges                 Get all cryptocurrency exchanges");
    println!("  exchange <exchange_id>    Get data for a specific exchange by ID");
    println!("  social <coin_id>          Get social media stats for a specific coin by ID");
}
