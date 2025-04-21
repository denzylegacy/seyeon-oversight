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
            initial_investment_fraction: 1.0,
            dca_buy_threshold: 0.05,
            dca_buy_fraction: 1.0,
            profit_sell_threshold: 0.10,
            profit_sell_fraction: 0.5,
            generic_fee: 0.005,
            buy_threshold: 5,
            sell_threshold: 2,
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
}

#[derive(Debug, Clone)]
pub struct Event {
    pub datetime: DateTime<Utc>,
    pub price: f64,
    pub signal: Signal,
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

    /// Generates buy and sell signals using the latest row of the final dataframe.
    pub fn generate_signal(&self, idx: usize) -> (bool, bool) {
        let price = self
            .final_df
            .column("price")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let ma25 = self
            .final_df
            .column("ma25")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let ma50 = self
            .final_df
            .column("ma50")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let ma5 = self
            .final_df
            .column("ma5")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let macd = self
            .final_df
            .column("macd")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let signal_val = self
            .final_df
            .column("signal")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let rsi = self
            .final_df
            .column("rsi")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let roc = self
            .final_df
            .column("roc")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap_or(f64::NAN);
        let lower_band = self
            .final_df
            .column("lower_band")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap_or(f64::NAN);
        let ma111 = self
            .final_df
            .column("ma111")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();
        let pi_cycle_top = self
            .final_df
            .column("pi_cycle_top")
            .unwrap()
            .f64()
            .unwrap()
            .get(idx)
            .unwrap();

        let buy_conditions = vec![
            price > ma25,
            price > ma50,
            ma5 > ma25,
            macd > signal_val,
            rsi < 40.0,
            roc > 0.0,
            price <= lower_band * 1.02,
            ma111 < pi_cycle_top,
            self.fgi <= 44,
        ];
        let buy_count = buy_conditions.iter().filter(|&&c| c).count();

        let sell_conditions = vec![
            rsi > 60.0,
            macd < signal_val,
            ma111 > pi_cycle_top,
            self.fgi >= 56,
        ];
        let sell_count = sell_conditions.iter().filter(|&&c| c).count();

        let buy_threshold = if self.fgi != 50 {
            self.params.buy_threshold + 1
        } else {
            self.params.buy_threshold
        };
        let sell_threshold = if self.fgi != 50 {
            self.params.sell_threshold + 1
        } else {
            self.params.sell_threshold
        };

        let buy_signal = buy_count >= buy_threshold;
        let sell_signal = sell_count >= sell_threshold;

        (buy_signal, sell_signal)
    }

    /// Runs in “production” mode (using only the most recent row).
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
        let (buy_signal, sell_signal) = self.generate_signal(idx);
        if buy_signal && !sell_signal {
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
            if price < pos.avg_price * (1.0 - self.params.dca_buy_threshold)
                && self.current_cash > 100.0
            {
                let dt_val = self
                    .final_df
                    .column("datetime")
                    .unwrap()
                    .i64()
                    .unwrap()
                    .get(idx)
                    .unwrap();
                let datetime = Utc.timestamp_millis_opt(dt_val).unwrap();
                let investment = self.current_cash * self.params.dca_buy_fraction;
                if investment < 50.0 {
                    return;
                }
                let fee = investment * self.params.generic_fee;
                let amount = (investment - fee) / price;
                let total_amount = pos.amount + amount;
                pos.avg_price = ((pos.avg_price * pos.amount) + (price * amount)) / total_amount;
                pos.amount = total_amount;
                pos.investment += investment;
                self.current_cash -= investment;
                self.held += amount;
                self.trade_history.push(Trade {
                    trade_type: TradeType::DcaBuy,
                    datetime,
                    price,
                    amount,
                });
            }
        }
    }

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
                let sell_amount = pos.amount * self.params.profit_sell_fraction;
                let investment_value = sell_amount * price;
                let fee = investment_value * self.params.generic_fee;
                let proceeds = investment_value - fee;
                self.current_cash += proceeds;
                self.held -= sell_amount;
                pos.amount -= sell_amount;
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
            let final_price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
            let sell_price = if final_price < pos.avg_price {
                pos.avg_price
            } else {
                final_price
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

    /// Runs the simulation over the last 365 data points (or the full dataset if shorter).
    pub fn run_simulation(&mut self) {
        let total_data = self.final_df.height();
        let start_idx = if total_data > 365 {
            total_data - 365
        } else {
            0
        };
        let mut in_position = false;

        for idx in start_idx..total_data {
            let price = self
                .final_df
                .column("price")
                .unwrap()
                .f64()
                .unwrap()
                .get(idx)
                .unwrap();
            let (buy_signal, sell_signal) = self.generate_signal(idx);

            if !in_position {
                if buy_signal && self.current_cash > 50.0 {
                    self.enter_trade(idx, self.params.initial_investment_fraction);
                    in_position = true;
                }
            } else {
                self.dca_buy(idx);
                self.partial_sell(idx);
                if sell_signal {
                    if let Some(pos) = &self.position {
                        if price > pos.avg_price {
                            self.full_sell(idx);
                            in_position = false;
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
}
