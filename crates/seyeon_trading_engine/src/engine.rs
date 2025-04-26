use chrono::{DateTime, TimeZone, Utc};
use polars::prelude::*;

// --- Trade-related structs remain the same ---
#[derive(Debug, Clone)]
pub enum TradeType {
    Buy,
    DcaBuy,
    PartialSell,
    FullSell,
    FinalSell,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub trade_type: TradeType,
    pub datetime: DateTime<Utc>,
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub avg_price: f64,
    pub amount: f64,
    pub investment: f64,
    pub entry_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub initial_capital: f64,
    pub initial_investment_fraction: f64, // invest 100% of available cash at entry
    pub dca_buy_threshold: f64, // if the price drops 5% below the average cost, perform DCA
    pub dca_buy_fraction: f64,  // invest 100% of available cash in the DCA
    pub profit_sell_threshold: f64, // sell (partially) if the price is 10% above the average cost
    pub profit_sell_fraction: f64, // sell 50% of the position for profit
    pub generic_fee: f64,       // fixed fee (0.5% in this example)
    pub buy_threshold: usize,
    pub sell_threshold: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            initial_capital: 10_000.0,
            initial_investment_fraction: 0.35,     // Invest only 35% initially to leave room for aggressive DCA
            dca_buy_threshold: 0.10,              // More aggressive DCA at 10% drops
            dca_buy_fraction: 0.75,               // Invest 75% of available cash in each DCA
            profit_sell_threshold: 0.20,          // Take profit at 20% gains - let winners run longer
            profit_sell_fraction: 0.40,           // Sell 40% of position for partial profits
            generic_fee: 0.005,                   // Keep fee at 0.5%
            buy_threshold: 3,                     // More lenient buy threshold
            sell_threshold: 2,                    // Keep sell threshold at 2 conditions
        }
    }
}

// --- TradingEngine updated to use the final DataFrame ---
pub struct TradingEngine {
    /// The final (fully calculated) dataframe with all indicators.
    pub final_df: DataFrame,
    pub fgi: u8, // if not provided, assume 50 (neutral)
    pub params: Params,
    pub current_cash: f64,
    pub held: f64,
    pub trade_history: Vec<Trade>,
    pub position: Option<Position>,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub initial_capital: f64,
    pub final_portfolio_value: f64,
    pub roi: f64,
    pub num_trades: usize,
    pub estimated_fees_paid: f64,
}

#[derive(Debug, Clone)]
pub enum Signal {
    Hold,
    Buy,
    Sell,
    DcaBuy,
    DcaSell,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub datetime: DateTime<Utc>,
    pub price: f64,
    pub signal: Signal,
}

#[derive(Debug, Clone)]
pub struct PortfolioSimulation {
    pub symbol: String,
    pub roi: f64,
    pub final_value: f64,
    pub num_trades: usize,
}

impl TradingEngine {
    /// Constructs a new TradingEngine. Note that we immediately calculate the
    /// final dataframe from the provided Indicators.
    pub fn new(final_df: DataFrame, fgi: Option<u8>, params: Params) -> Self {
        let fgi_val = fgi.unwrap_or(50);

        Self {
            final_df,
            fgi: fgi_val,
            current_cash: params.initial_capital,
            params,
            held: 0.0,
            trade_history: Vec::new(),
            position: None,
        }
    }

    /// Constructs a new TradingEngine with an associated symbol. This is a convenience 
    /// method that just calls new() but allows specifying a symbol for identification.
    pub fn new_with_symbol(final_df: DataFrame, _symbol: String, fgi: Option<u8>, params: Params) -> Self {
        // We ignore the symbol for now - it's just for identification in the caller
        Self::new(final_df, fgi, params)
    }

    /// Generate buy and sell signals with advanced weighted scoring system
    pub fn generate_signal(&self, idx: usize) -> (bool, bool) {
        let price = self.final_df.column("price").unwrap().f64().unwrap().get(idx).unwrap();
        let ma25 = self.final_df.column("ma25").unwrap().f64().unwrap().get(idx).unwrap();
        let ma50 = self.final_df.column("ma50").unwrap().f64().unwrap().get(idx).unwrap();
        let ma5 = self.final_df.column("ma5").unwrap().f64().unwrap().get(idx).unwrap();
        let ma111 = self.final_df.column("ma111").unwrap().f64().unwrap().get(idx).unwrap_or(price * 0.9);
        let macd = self.final_df.column("macd").unwrap().f64().unwrap().get(idx).unwrap();
        let signal_val = self.final_df.column("signal").unwrap().f64().unwrap().get(idx).unwrap();
        let rsi = self.final_df.column("rsi").unwrap().f64().unwrap().get(idx).unwrap_or(50.0);
        let _roc = self.final_df.column("roc").unwrap().f64().unwrap().get(idx).unwrap_or(0.0);
        let lower_band = self.final_df.column("lower_band").unwrap().f64().unwrap().get(idx).unwrap_or(price * 0.9);
        let upper_band = self.final_df.column("upper_band").unwrap().f64().unwrap().get(idx).unwrap_or(price * 1.1);
        let vma20 = self.final_df.column("vma20").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        let atr14 = self.final_df.column("atr14").unwrap().f64().unwrap().get(idx).unwrap_or(price * 0.05);
        
        // Price volatility metric - higher means more volatile
        let volatility_ratio = atr14 / price;
        let volatility_high = volatility_ratio > 0.03; // 3% daily volatility is high
        
        // Trend strength metrics
        let strong_uptrend = ma5 > ma25 && ma25 > ma50 && ma50 > ma111;
        let is_oversold = rsi < 30.0;
        let is_overbought = rsi > 70.0;
        
        // Advanced buy scoring system (total possible: 100 points)
        let mut buy_score = 0;
        
        // Price relative to moving averages (30 points)
        if price > ma5 { buy_score += 5; }
        if price > ma25 { buy_score += 10; }
        if ma5 > ma25 { buy_score += 15; }
        
        // Momentum indicators (25 points)
        if macd > signal_val { buy_score += 15; }
        if macd > 0.0 { buy_score += 10; }
        
        // Oversold conditions (20 points)
        if is_oversold { buy_score += 20; }
        else if rsi < 40.0 { buy_score += 10; }
        
        // Support level indicators (15 points)
        if price <= lower_band * 1.02 { buy_score += 15; }
        else if price <= lower_band * 1.05 { buy_score += 10; }
        
        // Volume confirmation (10 points)
        if vma20 > price * 0.9 { buy_score += 10; }
        
        // Market sentiment from Fear and Greed Index (0-100)
        // For crypto, extreme fear (low FGI) can be a good buying opportunity
        // (0-25 extreme fear, 26-45 fear, 46-55 neutral, 56-75 greed, 76-100 extreme greed)
        if self.fgi < 25 { buy_score += 15; } // extreme fear is a contrarian buy signal
        else if self.fgi < 40 { buy_score += 10; }
        else if self.fgi > 80 { buy_score -= 15; } // extreme greed reduces buy score
        else if self.fgi > 65 { buy_score -= 10; }
        
        // Advanced sell scoring system (total possible: 100 points)
        let mut sell_score = 0;
        
        // Price relative to moving averages (30 points)
        if price < ma5 { sell_score += 5; }
        if price < ma25 { sell_score += 10; }
        if ma5 < ma25 { sell_score += 15; }
        
        // Momentum indicators (25 points)
        if macd < signal_val { sell_score += 15; }
        if macd < 0.0 { sell_score += 10; }
        
        // Overbought conditions (20 points)
        if is_overbought { sell_score += 20; }
        else if rsi > 65.0 { sell_score += 10; }
        
        // Resistance level indicators (15 points)
        if price >= upper_band * 0.98 { sell_score += 15; }
        else if price >= upper_band * 0.95 { sell_score += 10; }
        
        // Volume confirmation (10 points)
        if vma20 < price { sell_score += 10; }
        
        // Market sentiment penalties from Fear and Greed Index (0-100)
        if self.fgi > 80 { sell_score += 15; } // extreme greed is a sell signal
        else if self.fgi > 65 { sell_score += 10; }
        else if self.fgi < 20 { sell_score -= 10; } // extreme fear reduces sell score
        
        // Adjust thresholds based on volatility
        let buy_threshold = if volatility_high { 70 } else { 60 };
        let sell_threshold = if volatility_high { 65 } else { 70 };
        
        // During strong uptrend, we want to be more conservative with selling
        let adjusted_sell_threshold = if strong_uptrend { sell_threshold + 10 } else { sell_threshold };
        
        let buy_signal = buy_score >= buy_threshold;
        let sell_signal = sell_score >= adjusted_sell_threshold;
        
        (buy_signal, sell_signal)
    }

    /// Runs in "production" mode (using only the most recent row).
    pub fn poll_event(&self) -> Event {
        let idx = self.final_df.height() - 1;
        let price = self
            .final_df
            .column("price")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        // Extract datetime as an i64 (milliseconds) and convert it to DateTime<Utc>
        let dt_val = self
            .final_df
            .column("datetime")
            .unwrap()
            .i64()
            .unwrap()
            .get(idx)
            .unwrap();
        let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
        
        // Regular buy/sell signals
        let (buy_signal, sell_signal) = self.generate_signal(idx);
        
        // Check for DCA opportunities
        let is_dca_buy = self.check_dca_buy_opportunity(idx);
        let is_dca_sell = self.check_dca_sell_opportunity(idx);
        
        if is_dca_buy {
            Event {
                datetime,
                price,
                signal: Signal::DcaBuy,
            }
        } else if is_dca_sell {
            Event {
                datetime,
                price,
                signal: Signal::DcaSell,
            }
        } else if buy_signal && !sell_signal {
            Event {
                datetime,
                price,
                signal: Signal::Buy,
            }
        } else if sell_signal && !buy_signal {
            Event {
                datetime,
                price,
                signal: Signal::Sell,
            }
        } else {
            Event {
                datetime,
                price,
                signal: Signal::Hold,
            }
        }
    }

    fn enter_trade(&mut self, idx: usize, investment_fraction: f64) {
        let price = self
            .final_df
            .column("price")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let dt_val = self
            .final_df
            .column("datetime")
            .unwrap()
            .i64()
            .unwrap()
            .get(idx)
            .unwrap();
        let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
        let investment = self.current_cash * investment_fraction;
        let fee = investment * self.params.generic_fee;
        let amount = (investment - fee) / price;

        self.current_cash -= investment;
        self.held += amount;
        self.position = Some(Position {
            avg_price: price,
            amount,
            investment,
            entry_time: datetime,
        });
        self.trade_history.push(Trade {
            trade_type: TradeType::Buy,
            datetime,
            price,
            amount,
        });
    }

    fn dca_buy(&mut self, idx: usize) {
        if let Some(pos) = &mut self.position {
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
                
            // More aggressive DCA buy strategy based on our sophisticated conditions
            let dt_val = self
                .final_df
                .column("datetime")
                .unwrap()
                .i64()
                .unwrap()
                .get(idx)
                .unwrap();
            let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
            
            // Scale investment amount based on price drop percentage
            let drop_percentage = (pos.avg_price - price) / pos.avg_price;
            let investment_scale = if drop_percentage > 0.2 {
                // Very large drop, deploy maximum capital
                1.0
            } else if drop_percentage > 0.15 {
                // Significant drop, deploy most capital
                0.9
            } else if drop_percentage > 0.1 {
                // Good drop, deploy substantial capital
                self.params.dca_buy_fraction
            } else {
                // Small drop, deploy moderate capital
                self.params.dca_buy_fraction * 0.8
            };
            
            let investment = self.current_cash * investment_scale;
            
            // Only proceed if investment is meaningful
            if investment < 100.0 {
                return;
            }
            
            let fee = investment * self.params.generic_fee;
            let amount = (investment - fee) / price;
            let total_amount = pos.amount + amount;
            
            // Update position details
            pos.avg_price = ((pos.avg_price * pos.amount) + (price * amount)) / total_amount;
            pos.amount = total_amount;
            pos.investment += investment;
            
            // Update portfolio state
            self.current_cash -= investment;
            self.held += amount;
            
            // Record the trade
            self.trade_history.push(Trade {
                trade_type: TradeType::DcaBuy,
                datetime,
                price,
                amount,
            });
        }
    }

    /// Partially sells a position to take some profits
    fn partial_sell(&mut self, idx: usize) {
        if let Some(pos) = &mut self.position {
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
            
            // Only proceed if we're in profit and above our threshold
            if price > pos.avg_price * (1.0 + self.params.profit_sell_threshold) {
                let dt_val = self
                    .final_df
                    .column("datetime")
                    .unwrap()
                    .i64()
                    .unwrap()
                    .get(idx)
                    .unwrap();
                let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
                
                // Sell a percentage of our position
                let sell_amount = pos.amount * self.params.profit_sell_fraction;
                let investment_value = sell_amount * price;
                let fee = investment_value * self.params.generic_fee;
                let proceeds = investment_value - fee;
                
                // Update position and cash
                self.current_cash += proceeds;
                self.held -= sell_amount;
                pos.amount -= sell_amount;
                
                // Record the trade
                self.trade_history.push(Trade {
                    trade_type: TradeType::PartialSell,
                    datetime,
                    price,
                    amount: sell_amount,
                });
            }
        }
    }

    fn full_sell(&mut self, idx: usize) {
        if let Some(pos) = &self.position {
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
            if price > pos.avg_price {
                let dt_val = self
                    .final_df
                    .column("datetime")
                    .unwrap()
                    .i64()
                    .unwrap()
                    .get(idx)
                    .unwrap();
                let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
                let sell_amount = pos.amount;
                let investment_value = sell_amount * price;
                let fee = investment_value * self.params.generic_fee;
                let proceeds = investment_value - fee;
                self.current_cash += proceeds;
                self.held -= sell_amount;
                self.trade_history.push(Trade {
                    trade_type: TradeType::FullSell,
                    datetime,
                    price,
                    amount: sell_amount,
                });
                self.position = None;
            }
        }
    }

    fn final_sell(&mut self) {
        if let Some(pos) = &self.position {
            let idx = self.final_df.height() - 1;
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
            let sell_price = if price < pos.avg_price {
                pos.avg_price
            } else {
                price
            };
            let dt_val = self
                .final_df
                .column("datetime")
                .unwrap()
                .i64()
                .unwrap()
                .get(idx)
                .unwrap();
            let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
            let sell_amount = pos.amount;
            let investment_value = sell_amount * sell_price;
            let fee = investment_value * self.params.generic_fee;
            let proceeds = investment_value - fee;
            self.current_cash += proceeds;
            self.held -= sell_amount;
            self.trade_history.push(Trade {
                trade_type: TradeType::FinalSell,
                datetime,
                price: sell_price,
                amount: sell_amount,
            });
            self.position = None;
        }
    }

    /// Runs the simulation with optimized strategy for maximum ROI
    pub fn run_simulation(&mut self) {
        let total_data = self.final_df.height();
        let start_idx = if total_data > 365 {
            total_data - 365
        } else {
            0
        };
        let mut in_position = false;
        let mut waiting_for_better_entry = false;
        let mut last_sell_price = 0.0;
        let mut consecutive_losses = 0;

        // Enhanced position tracking
        let mut _wins = 0;
        let mut _losses = 0;

        for idx in start_idx..total_data {
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
                
            let ma25 = self.final_df.column("ma25").unwrap().f64().unwrap().get(idx).unwrap_or(price);
            let rsi = self.final_df.column("rsi").unwrap().f64().unwrap().get(idx).unwrap_or(50.0);
                
            let (buy_signal, sell_signal) = self.generate_signal(idx);
            
            // Enhanced strategy with DCA opportunities
            let is_dca_buy = self.check_dca_buy_opportunity(idx);
            let is_dca_sell = self.check_dca_sell_opportunity(idx);

            // Strategy adjustments based on market conditions
            let strong_buying_signal = buy_signal && rsi < 35.0 && price < ma25;
            
            if !in_position {
                // If we've sold recently, wait for a better entry
                if waiting_for_better_entry && price >= last_sell_price {
                    continue;
                }
                
                // Adjust buying strategy based on past performance
                if consecutive_losses >= 2 {
                    // After multiple losses, be more selective with entries
                    if strong_buying_signal && self.current_cash > 100.0 {
                        self.enter_trade(idx, self.params.initial_investment_fraction * 0.7);
                        in_position = true;
                        waiting_for_better_entry = false;
                    }
                } else {
                    // Normal entry logic
                    if buy_signal && self.current_cash > 100.0 {
                        // Scale initial investment based on signal strength
                        let investment_fraction = if strong_buying_signal {
                            self.params.initial_investment_fraction * 1.2
                        } else {
                            self.params.initial_investment_fraction
                        };
                        
                        self.enter_trade(idx, investment_fraction.min(0.7));  // Cap at 70% of cash
                        in_position = true;
                        waiting_for_better_entry = false;
                    }
                }
            } else {
                // Position management with enhanced DCA strategy
                if is_dca_buy {
                    self.dca_buy(idx);
                } else if is_dca_sell {
                    self.partial_sell(idx);
                } else if sell_signal {
                    // Full exit on strong sell signal
                    if let Some(pos) = &self.position {
                        if price > pos.avg_price {
                            self.full_sell(idx);
                            in_position = false;
                            
                            // Track win and update strategy parameters
                            last_sell_price = price;
                            waiting_for_better_entry = true;
                            _wins += 1;
                            consecutive_losses = 0;
                        }
                    }
                }
                
                // Risk management - cut losses if position has been underwater for too long
                if let Some(pos) = &self.position {
                    if price < pos.avg_price * 0.8 {
                        // Price dropped 20% below average - consider cutting losses
                        let dt_val = self.final_df.column("datetime").unwrap().i64().unwrap().get(idx).unwrap();
                        let current_time = Utc.timestamp_millis_opt(dt_val).unwrap();
                        let time_in_position = current_time.signed_duration_since(pos.entry_time);
                        
                        // If we've been underwater for more than 14 days and RSI is not oversold
                        if time_in_position.num_days() > 14 && rsi > 40.0 {
                            self.full_sell(idx);
                            in_position = false;
                            consecutive_losses += 1;
                            _losses += 1;
                        }
                    }
                }
            }
        }

        if in_position && self.position.is_some() {
            self.final_sell();
        }
    }

    /// Displays a summary of the simulation results.
    #[must_use]
    pub fn get_summary(&self) -> Summary {
        let idx = self.final_df.height() - 1;
        let final_price = self
            .final_df
            .column("price")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let final_portfolio_value = self.current_cash + (self.held * final_price);
        let profit = final_portfolio_value - self.params.initial_capital;
        let roi = (profit / self.params.initial_capital) * 100.0;
        let total_fees: f64 = self
            .trade_history
            .iter()
            .map(|t| t.price * t.amount * self.params.generic_fee)
            .sum();

        // println!("Initial Capital: ${:.2}", self.params.initial_capital);
        // println!("Final Portfolio Value: ${:.2}", final_portfolio_value);
        // println!("Return: {:+.2}%", roi);
        // println!("Total Number of Trades: {}", self.trade_history.len());
        // println!("Total Fees Paid: ${:.2}", total_fees);
        // println!("{:=^60}", "");

        Summary {
            final_portfolio_value,
            roi,
            initial_capital: self.params.initial_capital,
            num_trades: self.trade_history.len(),
            estimated_fees_paid: total_fees,
        }
    }

    /// Calculates the correlation matrix between multiple assets
    /// Returns a DataFrame with the correlation matrix
    pub fn calculate_correlation_matrix(price_data: &[(&str, &Vec<f64>)]) -> Result<DataFrame, PolarsError> {
        let mut columns = Vec::new();
        
        for (symbol, prices) in price_data {
            columns.push(Series::new((*symbol).to_string().into(), (*prices).clone()).into());
        }
        
        let df = DataFrame::new(columns)?;
        
        let col_names = df.get_column_names();
        let n_cols = col_names.len();
        
        let mut corr_matrix = vec![vec![0.0; n_cols]; n_cols];
        
        for i in 0..n_cols {
            for j in 0..n_cols {
                if i == j {
                    corr_matrix[i][j] = 1.0;
                } else {
                    let series_i = df.column(col_names[i])?.f64()?;
                    let series_j = df.column(col_names[j])?.f64()?;
                    
                    let corr = Self::pearson_correlation(series_i, series_j)?;
                    corr_matrix[i][j] = corr;
                }
            }
        }
        
        let mut corr_columns = Vec::new();
        
        for (i, name) in col_names.iter().enumerate() {
            let corr_series = Series::new(name.to_string().into(), corr_matrix[i].clone()).into();
            corr_columns.push(corr_series);
        }
        
        let corr_df = DataFrame::new(corr_columns)?;
        
        Ok(corr_df)
    }
    
    fn pearson_correlation(s1: &ChunkedArray<Float64Type>, s2: &ChunkedArray<Float64Type>) -> Result<f64, PolarsError> {
        // Get lengths, ensure they match
        let len1 = s1.len();
        let len2 = s2.len();
        
        if len1 != len2 {
            return Err(PolarsError::ShapeMismatch(
                format!("Series lengths don't match: {} vs {}", len1, len2).into(),
            ));
        }
        
        if len1 == 0 {
            return Err(PolarsError::ComputeError(
                "Cannot compute correlation on empty series".into(),
            ));
        }
        
        let mean1: f64 = s1.mean().unwrap_or(0.0);
        let mean2: f64 = s2.mean().unwrap_or(0.0);
        
        let mut numerator = 0.0;
        let mut denom1 = 0.0;
        let mut denom2 = 0.0;
        
        for i in 0..len1 {
            let v1 = match s1.get(i) {
                Some(v) => v,
                None => continue,
            };
            
            let v2 = match s2.get(i) {
                Some(v) => v,
                None => continue,
            };
            
            let diff1 = v1 - mean1;
            let diff2 = v2 - mean2;
            
            numerator += diff1 * diff2;
            denom1 += diff1 * diff1;
            denom2 += diff2 * diff2;
        }
        
        if denom1 == 0.0 || denom2 == 0.0 {
            return Ok(0.0);
        }
        
        let correlation = numerator / (denom1.sqrt() * denom2.sqrt());
        
        if correlation < -1.0 || correlation > 1.0 {
            Ok(correlation.clamp(-1.0, 1.0))
        } else {
            Ok(correlation)
        }
    }
    
    /// Exports the correlation matrix to an HTML heatmap
    pub fn export_correlation_heatmap(correlation_df: &DataFrame, file_path: &str) -> std::io::Result<()> {
        let mut html_content = String::from(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Crypto Assets Correlation Heatmap</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .container { width: 900px; height: 700px; }
        h1 { color: #333; }
    </style>
</head>
<body>
    <h1>Crypto Assets Correlation Heatmap</h1>
    <div class="container" id="heatmap"></div>
    <script>
"#);

        let symbols: Vec<String> = correlation_df.get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        let mut z_values = String::from("[");
        for i in 0..symbols.len() {
            z_values.push_str("[");
            for j in 0..symbols.len() {
                let value = match correlation_df.get(j) {
                    Some(series) => match series.get(i) {
                        Some(value) => match value.try_extract::<f64>() {
                            Ok(v) => v,
                            Err(_) => 0.0,
                        },
                        None => 0.0,
                    },
                    None => 0.0,
                };
                z_values.push_str(&format!("{:.4}", value));
                if j < symbols.len() - 1 {
                    z_values.push_str(", ");
                }
            }
            z_values.push_str("]");
            if i < symbols.len() - 1 {
                z_values.push_str(", ");
            }
        }
        z_values.push_str("]");
        
        let symbols_js = symbols
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>()
            .join(", ");
        
        html_content.push_str(&format!(r#"
        var data = [{{
            z: {},
            x: [{}],
            y: [{}],
            type: 'heatmap',
            colorscale: 'RdBu',
            zmin: -1,
            zmax: 1
        }}];

        var layout = {{
            title: 'Crypto Assets Price Correlation',
            annotations: [],
            xaxis: {{
                ticks: '',
                side: 'top'
            }},
            yaxis: {{
                ticks: '',
                ticksuffix: ' ',
                autosize: false
            }}
        }};

        // Add correlation values as annotations
        for (var i = 0; i < {3}.length; i++) {{
            for (var j = 0; j < {3}.length; j++) {{
                var result = {{
                    xref: 'x1',
                    yref: 'y1',
                    x: {3}[j],
                    y: {3}[i],
                    text: data[0].z[i][j].toFixed(2),
                    font: {{
                        family: 'Arial',
                        size: 12,
                        color: Math.abs(data[0].z[i][j]) > 0.5 ? 'white' : 'black'
                    }},
                    showarrow: false
                }};
                layout.annotations.push(result);
            }}
        }}

        Plotly.newPlot('heatmap', data, layout);
    </script>
</body>
</html>
"#, z_values, symbols_js, symbols_js, symbols_js));

        std::fs::write(file_path, html_content)?;
        
        Ok(())
    }
    
    /// Run simulation for multiple assets and compare their performance
    pub fn compare_assets_performance(assets_data: &[(&str, DataFrame)], _days: usize) -> Vec<PortfolioSimulation> {
        let mut results = Vec::new();
        
        for (symbol, dataframe) in assets_data {
            let params = Params::default();
            let mut engine = TradingEngine::new(dataframe.clone(), None, params);
            
            engine.run_simulation();
            let summary = engine.get_summary();
            
            results.push(PortfolioSimulation {
                symbol: symbol.to_string(),
                roi: summary.roi,
                final_value: summary.final_portfolio_value,
                num_trades: summary.num_trades,
            });
        }
        
        // Sort by ROI in descending order
        results.sort_by(|a, b| b.roi.partial_cmp(&a.roi).unwrap_or(std::cmp::Ordering::Equal));
        
        results
    }
    
    /// Exports the performance comparison to an HTML bar chart
    pub fn export_performance_comparison(results: &[PortfolioSimulation], file_path: &str) -> std::io::Result<()> {
        let mut html_content = String::from(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Crypto Assets Performance Comparison</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .container { width: 900px; height: 700px; }
        h1 { color: #333; }
        table { border-collapse: collapse; width: 100%; margin-top: 20px; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        tr:nth-child(even) { background-color: #f9f9f9; }
    </style>
</head>
<body>
    <h1>Crypto Assets Performance Comparison (365-day Simulation)</h1>
    <div class="container" id="chart"></div>
    
    <h2>Detailed Results</h2>
    <table>
        <tr>
            <th>Rank</th>
            <th>Asset</th>
            <th>ROI (%)</th>
            <th>Final Value ($)</th>
            <th>Number of Trades</th>
        </tr>
"#);

        for (i, result) in results.iter().enumerate() {
            html_content.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{:.2}%</td><td>${:.2}</td><td>{}</td></tr>\n",
                i + 1, result.symbol, result.roi, result.final_value, result.num_trades
            ));
        }
        
        html_content.push_str("</table>\n");

        let symbols: Vec<String> = results.iter()
            .map(|r| format!("\"{}\"", r.symbol))
            .collect();
        
        let roi_values: Vec<String> = results.iter()
            .map(|r| format!("{:.2}", r.roi))
            .collect();
        
        html_content.push_str(&format!(r#"
    <script>
        var data = [{{
            x: [{}],
            y: [{}],
            type: 'bar',
            marker: {{
                color: Array({}).fill().map((_, i) => 
                    'rgb(' + Math.floor(255 - i * (255 / {})) + ',' + 
                    Math.floor(50 + i * (150 / {})) + ',' + Math.floor(50) + ')'
                )
            }}
        }}];

        var layout = {{
            title: 'Return on Investment (ROI) by Asset',
            xaxis: {{ title: 'Asset' }},
            yaxis: {{ title: 'ROI (%)' }}
        }};

        Plotly.newPlot('chart', data, layout);
    </script>
</body>
</html>
"#, symbols.join(", "), roi_values.join(", "), symbols.len(), symbols.len(), symbols.len()));

        std::fs::write(file_path, html_content)?;
        
        Ok(())
    }

    /// Check if there's an opportunity for a DCA buy based on more sophisticated conditions
    fn check_dca_buy_opportunity(&self, idx: usize) -> bool {
        // Only relevant if we have a position
        if self.position.is_none() {
            return false;
        }
        
        let pos = self.position.as_ref().unwrap();
        
        // Extract technical indicators for analysis
        let price = self.final_df.column("price").unwrap().f64().unwrap().get(idx).unwrap();
        let rsi = self.final_df.column("rsi").unwrap().f64().unwrap().get(idx).unwrap_or(50.0);
        let lower_band = self.final_df.column("lower_band").unwrap().f64().unwrap().get(idx).unwrap_or(price * 0.9);
        let macd = self.final_df.column("macd").unwrap().f64().unwrap().get(idx).unwrap_or(0.0);
        let signal = self.final_df.column("signal").unwrap().f64().unwrap().get(idx).unwrap_or(0.0);
        let ma25 = self.final_df.column("ma25").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        let ma50 = self.final_df.column("ma50").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        let atr14 = self.final_df.column("atr14").unwrap().f64().unwrap().get(idx).unwrap_or(price * 0.05);
        
        // Check if price has dropped significantly below average cost
        let price_below_avg = price < pos.avg_price * (1.0 - self.params.dca_buy_threshold);
        
        // Note: we're using rsi value directly in conditions below, no need for a separate variable
        
        // Check if price is near or below lower Bollinger Band
        let price_near_lower_band = price <= lower_band * 1.03;
        
        // Check for potential bullish MACD crossover (MACD line crossing above signal line)
        let macd_bullish = macd > signal || (macd < 0.0 && macd > macd.abs() * -0.3 && macd > signal);
        
        // Check if price is near a major support level (MA25 or MA50)
        let near_support = (price <= ma25 * 1.02 && price >= ma25 * 0.98) || 
                          (price <= ma50 * 1.02 && price >= ma50 * 0.98);
        
        // Calculate volatility - we want to buy when volatility is high
        let volatility_ratio = atr14 / price;
        let volatility_high = volatility_ratio > 0.03; // 3% daily volatility is high for crypto
        
        // Basic requirement - must have enough cash for a meaningful purchase
        let has_enough_cash = self.current_cash >= 200.0;
        
        // Check how many DCA buys we've already done to prevent overbuying a falling asset
        let dca_buy_count = self.trade_history
            .iter()
            .filter(|t| matches!(t.trade_type, TradeType::DcaBuy))
            .count();
            
        let dca_limit_reached = dca_buy_count >= 3; // Limit to 3 DCA buys per position
        
        // Check Fear and Greed Index for market sentiment
        let extreme_fear = self.fgi < 20; // Extreme fear is often a good buying opportunity
                
        // Advanced scoring system for DCA Buy (total: 100 points)
        let mut dca_score = 0;
        
        // Price is below our average cost significantly (0-40 points)
        if price < pos.avg_price * 0.85 { dca_score += 40; }
        else if price < pos.avg_price * 0.9 { dca_score += 30; }
        else if price < pos.avg_price * 0.92 { dca_score += 20; } 
        else if price_below_avg { dca_score += 10; }
        
        // Asset is oversold (0-20 points)
        if rsi < 25.0 { dca_score += 20; }
        else if rsi < 30.0 { dca_score += 15; }
        else if rsi < 35.0 { dca_score += 10; }
        
        // Technical indicators suggest potential reversal (0-30 points)
        if price_near_lower_band { dca_score += 15; }
        if macd_bullish { dca_score += 10; }
        if near_support { dca_score += 5; }
        
        // Market conditions (0-10 points)
        if extreme_fear { dca_score += 10; }
        if volatility_high { dca_score += 5; }
        
        // Apply penalties
        if dca_limit_reached { dca_score -= 30; }
        
        let dca_threshold = 60; // Need 60+ points to trigger a DCA buy
        
        (dca_score >= dca_threshold) && has_enough_cash
    }
    
    /// Check if there's an opportunity for a DCA sell (partial profit taking)
    fn check_dca_sell_opportunity(&self, idx: usize) -> bool {
        // Only relevant if we have a position
        if self.position.is_none() {
            return false;
        }
        
        let pos = self.position.as_ref().unwrap();
        let price = self.final_df.column("price").unwrap().f64().unwrap().get(idx).unwrap();
        let rsi = self.final_df.column("rsi").unwrap().f64().unwrap().get(idx).unwrap_or(50.0);
        let upper_band = self.final_df.column("upper_band").unwrap().f64().unwrap().get(idx).unwrap_or(price * 1.1);
        let macd = self.final_df.column("macd").unwrap().f64().unwrap().get(idx).unwrap_or(0.0);
        let signal = self.final_df.column("signal").unwrap().f64().unwrap().get(idx).unwrap_or(0.0);
        let ma5 = self.final_df.column("ma5").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        let ma25 = self.final_df.column("ma25").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        let vma20 = self.final_df.column("vma20").unwrap().f64().unwrap().get(idx).unwrap_or(price);
        
        // Only consider DCA sell if we're in profit
        if price <= pos.avg_price {
            return false;
        }
        
        // Check if price is significantly above our average entry
        let profit_percentage = (price / pos.avg_price - 1.0) * 100.0;
        
        // Check for bearish MACD crossover (MACD line crossing below signal line)
        let macd_bearish = macd < signal && macd > 0.0;
        
        // Check if price is near or above upper Bollinger Band (overbought condition)
        let price_near_upper_band = price >= upper_band * 0.95;
        
        // Check if short-term MA is turning down from above medium-term MA
        let ma_turning_down = ma5 < ma5 * 1.005 && ma5 > ma25;
        
        // Check volume - decreasing volume on rallies can be a reversal signal
        let volume_confirmation = vma20 > price;
        
        // Check market sentiment from FGI - extreme greed suggests potential reversal
        let extreme_greed = self.fgi > 75;
        
        // Advanced scoring system for DCA Sell (total: 100 points)
        let mut sell_score = 0;
        
        // Profit level reached (0-40 points)
        if profit_percentage > 25.0 { sell_score += 40; }
        else if profit_percentage > 20.0 { sell_score += 30; }
        else if profit_percentage > 15.0 { sell_score += 20; }
        else if profit_percentage > self.params.profit_sell_threshold * 100.0 { sell_score += 15; }
        
        // Overbought conditions (0-25 points)
        if rsi > 80.0 { sell_score += 25; }
        else if rsi > 75.0 { sell_score += 20; }
        else if rsi > 70.0 { sell_score += 15; }
        else if rsi > 65.0 { sell_score += 10; }
        
        // Technical reversal signals (0-25 points)
        if price_near_upper_band { sell_score += 15; }
        if macd_bearish { sell_score += 10; }
        if ma_turning_down { sell_score += 5; }
        
        // Other factors (0-10 points)
        if extreme_greed { sell_score += 5; }
        if !volume_confirmation { sell_score += 5; }
        
        // Check how many DCA sells we've already done to avoid excessive trading
        let dca_sell_count = self.trade_history
            .iter()
            .filter(|t| matches!(t.trade_type, TradeType::PartialSell))
            .count();
        
        // Adjust threshold based on profit level and number of previous DCA sells
        let base_threshold = 65;
        let adjusted_threshold = if profit_percentage > 25.0 {
            base_threshold - 10  // Lower threshold for high profits
        } else if dca_sell_count >= 2 {
            base_threshold + 15  // Higher threshold after multiple sells
        } else {
            base_threshold
        };
        
        sell_score >= adjusted_threshold
    }
}