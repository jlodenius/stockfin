use anyhow::{Result, anyhow};
use yahoo_finance_api::YahooConnector;

pub struct StockApi {
    provider: YahooConnector,
}

impl Default for StockApi {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WeeklyRangeResponse {
    pub stock_name: String,
    pub prev_close: f64,
    pub last_close: f64,
}

impl StockApi {
    pub fn new() -> Self {
        let provider = YahooConnector::new().unwrap();
        Self { provider }
    }

    /// Returns the first and last close for a weekly span
    pub async fn weekly_range(&self, ticker: &str) -> Result<WeeklyRangeResponse> {
        // 1d = data_granularity
        // 5d = range
        match self.provider.get_quote_range(ticker, "1d", "5d").await {
            Ok(response) => {
                let meta = response.metadata()?;

                let stock_name = meta.short_name.as_ref().unwrap().clone();
                let prev_close = meta.chart_previous_close.unwrap();
                let last_close = response.last_quote()?.close;

                Ok(WeeklyRangeResponse {
                    stock_name,
                    prev_close,
                    last_close,
                })
            }
            Err(e) => Err(anyhow!("Error fetching {}: {}", ticker, e)),
        }
    }
}
