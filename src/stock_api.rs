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

pub struct RangeResponse {
    pub prev_close: f64,
    pub last_close: f64,
    pub pct_change: f64,
}

impl StockApi {
    pub fn new() -> Self {
        let provider = YahooConnector::new().unwrap();
        Self { provider }
    }

    /// Returns the first and last close for a weekly range
    pub async fn weekly_range(&self, ticker: &str) -> Result<RangeResponse> {
        self.range(ticker, "1d", "5d").await
    }

    /// Returns the first and last close for a daily range
    pub async fn daily_range(&self, ticker: &str) -> Result<RangeResponse> {
        self.range(ticker, "1d", "1d").await
    }

    /// Search for a ticker
    pub async fn search_ticker(&self, query: &str) -> Vec<(String, String)> {
        match self.provider.search_ticker(query).await {
            Ok(resp) => resp
                .quotes
                .iter()
                .map(|i| (i.symbol.clone(), i.short_name.clone()))
                .collect(),
            Err(_) => vec![],
        }
    }

    async fn range(
        &self,
        ticker: &str,
        data_granularity: &str,
        range: &str,
    ) -> Result<RangeResponse> {
        match self
            .provider
            .get_quote_range(ticker, data_granularity, range)
            .await
        {
            Ok(response) => {
                let meta = response.metadata()?;

                let prev_close = meta.chart_previous_close.unwrap();
                let last_close = response.last_quote()?.close;
                let pct_change = (last_close - prev_close) / prev_close;

                Ok(RangeResponse {
                    prev_close,
                    last_close,
                    pct_change,
                })
            }
            Err(e) => Err(anyhow!("Error fetching {}: {}", ticker, e)),
        }
    }
}
