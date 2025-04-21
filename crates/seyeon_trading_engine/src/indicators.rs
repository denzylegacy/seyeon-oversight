use crate::data_point::DataPoint;
use polars::prelude::*;
use std::ops::Mul;

pub struct Indicators {
    pub data: LazyFrame,
}

fn window(size: usize) -> RollingOptionsFixedWindow {
    RollingOptionsFixedWindow {
        window_size: size,
        ..Default::default()
    }
}

impl Indicators {
    pub fn new(data: Vec<DataPoint>) -> Self {
        let msx = data
            .iter()
            .map(|d| d.datetime.timestamp() * 1000)
            .collect::<Vec<_>>();
        let datetime = Column::new("datetime".into(), msx.clone());
        let at = Column::new("at".into(), msx)
            .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))
            .expect("Failed to cast column");

        let instance = Self {
            data: DataFrame::new(vec![
                Column::new(
                    "price".into(),
                    data.iter().map(|d| d.price).collect::<Vec<_>>(),
                ),
                Column::new(
                    "high".into(),
                    data.iter().map(|d| d.high).collect::<Vec<_>>(),
                ),
                Column::new("low".into(), data.iter().map(|d| d.low).collect::<Vec<_>>()),
                Column::new(
                    "open".into(),
                    data.iter().map(|d| d.open).collect::<Vec<_>>(),
                ),
                Column::new(
                    "volume".into(),
                    data.iter().map(|d| d.volume).collect::<Vec<_>>(),
                ),
                datetime.clone(),
                at,
            ])
            .expect("Failed to create DataFrame")
            .lazy(),
        };

        instance
    }

    fn calculate_ema(prices: Expr, span: usize) -> Expr {
        prices.ewm_mean(EWMOptions {
            alpha: 2.0 / (span as f64 + 1.0),
            ..Default::default()
        })
    }

    /// Calculate MACD
    /// - EMA12: 12-day Exponential Moving Average
    /// - EMA26: 26-day Exponential Moving Average
    /// - MACD: EMA12 - EMA26
    /// - Signal: 9-day EMA of MACD
    fn calculate_macd(frame: LazyFrame) -> LazyFrame {
        let ema12 = Self::calculate_ema(col("price"), 12).alias("ema12");
        let ema26 = Self::calculate_ema(col("price"), 26).alias("ema26");
        let macd = (col("ema12") - col("ema26")).alias("macd");
        let signal = Self::calculate_ema(col("macd"), 9).alias("signal");

        frame
            .with_column(ema12)
            .with_column(ema26)
            .with_column(macd)
            .with_column(signal)
    }

    /// Calculate Moving Averages
    /// - MA5: 5-day moving average
    /// - MA25: 25-day moving average
    /// - MA50: 50-day moving average
    /// - MA365: 365-day moving average
    /// - MA111: 111-day moving average
    /// - MA350: 350-day moving average
    pub fn calculate_moving_averages(frame: LazyFrame) -> LazyFrame {
        frame.with_columns([
            col("price").alias("ma5").rolling_mean(window(5)),
            col("price").alias("ma25").rolling_mean(window(25)),
            col("price").alias("ma50").rolling_mean(window(50)),
            col("price").alias("ma365").rolling_mean(window(365)),
            col("price").alias("ma111").rolling_mean(window(111)),
            col("price").alias("ma350").rolling_mean(window(350)),
        ])
    }

    /// Calculate Bollinger Bands
    /// - Upper Band: MA25 + 2 * std20
    /// - Lower Band: MA25 - 2 * std20
    /// - std20: 20-day rolling standard deviation
    fn calculate_bollinger_bands(frame: LazyFrame) -> LazyFrame {
        let std20 = col("price").rolling_std(window(20)).alias("std20");

        let upper_band_expr = (col("ma25") + (lit(2.0) * col("std20"))).alias("upper_band");
        let lower_band_expr = (col("ma25") - (lit(2.0) * col("std20"))).alias("lower_band");

        frame
            .with_column(std20)
            .with_columns_seq([upper_band_expr, lower_band_expr])
    }

    /// Calculates the Rate of Change (ROC) for a series of price data using Polars' LazyFrame.
    ///
    /// This function calculates the percentage change in price relative to the price
    /// `period` periods ago for each data point in the time series. The result is stored in
    /// the `roc` column, with a value of `NaN` for indices where there are not enough
    /// previous data points to calculate the ROC.
    /// # Behavior Example
    /// The function performs the following steps:
    /// 1. Uses Polars' lazy API to calculate the ROC using the formula:
    ///    ```
    ///    ROC = ((current_price / price_from_12_periods_ago) - 1.0) * 100
    ///    ```
    ///    where `current_price` is the price at the current index, and `price_from_12_periods_ago` is
    ///    the price 12 periods prior.
    /// 2. If there are fewer than 12 data points before the current index, it assigns `NaN` for the ROC value.
    /// 3. The calculated ROC values are stored in the `roc` column of the `LazyFrame`.
    ///
    fn calculate_roc(df: LazyFrame, period: i32) -> LazyFrame {
        // Define the lazy expression for calculating ROC
        let roc_expr = ((col("price") / col("price").shift(lit(period))) - lit(1.0)) * lit(100.0);

        // Apply the ROC calculation to the LazyFrame
        df.with_columns(vec![roc_expr.alias("roc")])
    }

    /// Computes the Volume-Weighted Moving Average (VWMA) over a rolling window of 20 data points.
    ///
    /// # Arguments
    /// * `df` - A `LazyFrame` containing at least two columns:
    ///   - `price`: The price data for the asset (type `Float64`).
    ///   - `volume`: The volume data for the asset (type `Float64`).
    ///
    /// # Returns
    /// A `LazyFrame` containing the calculated VWMA for each window, as a new column `vwma`.
    /// The VWMA is calculated by dividing the sum of `price * volume` by the sum of `volume` within each window.
    fn calculate_vwma(df: LazyFrame, period: usize) -> LazyFrame {
        let vwma = df
            // Create a new column for price * volume
            .with_columns(vec![(col("price") * col("volume")).alias("price_volume")])
            // Calculate the rolling sum of price * volume and volume
            .with_columns(vec![
                col("price_volume").rolling_sum(window(period)),
                col("volume").rolling_sum(window(period)),
            ])
            // Calculate the VWMA by dividing the sum of price * volume by the sum of volume
            .with_columns(vec![
                (col("price_volume") / col("volume")).alias(format!("vma{period}"))
            ]);

        vwma
    }

    /// Computes the Average True Range (ATR) over a rolling window of 14 data points.
    ///
    /// # Arguments
    /// * `df` - A `LazyFrame` containing at least one column:
    ///  - `price`: The price data for the asset (type `Float64`).
    ///
    /// # Returns
    /// A `LazyFrame` containing the calculated ATR for each window, as a new column `atr`.
    fn calculate_atr(df: LazyFrame, period: usize) -> LazyFrame {
        df.with_columns([
            (col("price") - col("price").shift(lit(1)))
                .abs()
                .alias(format!("tr{period}")),
            (col("price") - col("price").shift(lit(1)))
                .abs()
                .rolling_mean(window(period))
                .alias(format!("atr{period}")),
        ])
    }

    /// Calculates the Pi Cycle Top indicator by multiplying the 350-day moving average by 2.
    /// The result is stored in the `pi_cycle_top` column of the `LazyFrame`.
    fn calculate_pi_cycle(df: LazyFrame) -> LazyFrame {
        df.with_column(col("ma350").mul(lit(2.0)).alias("pi_cycle_top"))
    }

    /// Computes the All-Time High (ATH) over the entire dataset.
    ///
    /// # Arguments
    /// * `df` - A `LazyFrame` containing at least one column:
    ///   - `price`: The price data for the asset (type `Float64`).
    ///
    /// # Returns
    /// A `LazyFrame` containing the calculated ATH for each data point, as a new column `ath`.
    fn calculate_ath(df: LazyFrame) -> LazyFrame {
        df.with_column(col("price").cum_max(false).alias("ath"))
    }

    /// Computes the Relative Strength Index (RSI) over a rolling window of 14 data points.
    ///
    /// # Arguments
    /// * `df` - A `LazyFrame` containing at least one column:
    ///   - `price`: The price data for the asset (type `Float64`).
    ///
    /// # Returns
    /// A `LazyFrame` containing the calculated RSI for each data point, as a new column `rsi`.
    fn calculate_rsi(df: LazyFrame, period: usize) -> LazyFrame {
        // Calculate the price change for each data point

        df.with_column((col("price") - col("price").shift(lit(1))).alias("delta"))
            .with_columns_seq([
                // Calculate the gain and loss for each data point
                when(col("delta").gt(lit(0.0)))
                    .then(col("delta"))
                    .otherwise(lit(0.0))
                    .alias("gain"),
                when(col("delta").lt(lit(0.0)))
                    .then(col("delta").abs())
                    .otherwise(lit(0.0))
                    .alias("loss"),
            ])
            .with_columns([
                col("gain").rolling_mean(window(period)).alias("avg_gain"),
                col("loss").rolling_mean(window(period)).alias("avg_loss"),
            ])
            .with_column((col("avg_gain") / col("avg_loss")).alias("rs"))
            .with_column((lit(100.0) - (lit(100.0) / (lit(1.0) + col("rs")))).alias("rsi"))
    }

    pub fn calculate(self) -> PolarsResult<DataFrame> {
        let Self { data } = self;

        let data = Self::calculate_moving_averages(data);
        let data = Self::calculate_bollinger_bands(data);
        let data = Self::calculate_macd(data);
        let data = Self::calculate_roc(data, 12);
        let data = Self::calculate_vwma(data, 20);
        let data = Self::calculate_atr(data, 14);
        let data = Self::calculate_pi_cycle(data);
        let data = Self::calculate_ath(data);
        let data = Self::calculate_rsi(data, 14);

        let mut df = data.collect()?;

        /* Rechunk the DataFrame to optimize performance for subsequent operations.
         * This is a common practice in Polars to improve performance by reducing the number of chunks.
         * Also, this is also required to make xlsx export work. */
        df.rechunk_mut();

        Ok(df)
    }
}
