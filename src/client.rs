use anyhow::Result;
use reqwest::{Client, Method, Request};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::TargetConfig;
use crate::metrics::RequestResult;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    target: Arc<TargetConfig>,
}

impl HttpClient {
    pub fn new(target: TargetConfig, timeout: Duration, pool_max_idle: usize) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(pool_max_idle)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .build()?;

        Ok(Self {
            client,
            target: Arc::new(target),
        })
    }

    pub async fn execute(&self) -> RequestResult {
        let start = Instant::now();

        // Build request
        let method = Method::from_bytes(self.target.method.as_bytes())
            .unwrap_or(Method::GET);

        let mut request_builder = self.client
            .request(method, &self.target.url);

        // Add headers
        for (key, value) in &self.target.headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &self.target.body {
            request_builder = request_builder.body(body.clone());
        }

        // Execute request and measure
        let result = match request_builder.build() {
            Ok(request) => self.send_and_measure(request, start).await,
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(format!("Failed to build request: {}", e)),
            },
        };

        result
    }

    async fn send_and_measure(&self, request: Request, start: Instant) -> RequestResult {
        match self.client.execute(request).await {
            Ok(response) => {
                let duration = start.elapsed();
                let status = response.status();
                let success = status.is_success();

                RequestResult {
                    start_time: start,
                    duration,
                    status_code: Some(status.as_u16()),
                    success,
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", status.as_u16()))
                    },
                }
            }
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = if e.is_timeout() {
                    "Request timeout".to_string()
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else if e.is_request() {
                    format!("Request error: {}", e)
                } else {
                    format!("Error: {}", e)
                };

                RequestResult {
                    start_time: start,
                    duration,
                    status_code: None,
                    success: false,
                    error: Some(error_msg),
                }
            }
        }
    }
}
