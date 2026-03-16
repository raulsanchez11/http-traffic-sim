use anyhow::Result;
use reqwest::{Client, Method, Request};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::TargetConfig;
use crate::metrics::RequestResult;
use crate::target_selector::TargetSelector;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    mode: ClientMode,
}

#[derive(Clone)]
enum ClientMode {
    SingleTarget { target: Arc<TargetConfig> },
    MultiTarget { selector: Arc<TargetSelector> },
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
            mode: ClientMode::SingleTarget {
                target: Arc::new(target),
            },
        })
    }

    pub fn new_multi_target(
        selector: Arc<TargetSelector>,
        timeout: Duration,
        pool_max_idle: usize,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(pool_max_idle)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .build()?;

        Ok(Self {
            client,
            mode: ClientMode::MultiTarget { selector },
        })
    }

    pub async fn execute(&self) -> RequestResult {
        let start = Instant::now();

        // Select target based on mode
        let target = match &self.mode {
            ClientMode::SingleTarget { target } => target.clone(),
            ClientMode::MultiTarget { selector } => selector.select(),
        };

        // Build request
        let method = Method::from_bytes(target.method.as_bytes())
            .unwrap_or(Method::GET);

        let mut request_builder = self.client
            .request(method, &target.url);

        // Add headers
        for (key, value) in &target.headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &target.body {
            request_builder = request_builder.body(body.clone());
        }

        // Execute request and measure
        let result = match request_builder.build() {
            Ok(request) => self.send_and_measure(request, start, &target.id).await,
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(format!("Failed to build request: {}", e)),
                target_id: target.id.clone(),
            },
        };

        result
    }

    async fn send_and_measure(&self, request: Request, start: Instant, target_id: &str) -> RequestResult {
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
                    target_id: target_id.to_string(),
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
                    target_id: target_id.to_string(),
                }
            }
        }
    }

    /// Execute request and hold the connection for a specified duration (for connection flood)
    pub async fn execute_and_hold(&self, hold_duration: Duration) -> RequestResult {
        let result = self.execute().await;

        // Hold for the specified duration
        tokio::time::sleep(hold_duration).await;

        result
    }

    /// Open a raw TCP connection and send partial HTTP request (for slowloris)
    pub async fn send_partial_request(&self, url: &str, partial_headers: &str) -> Result<()> {
        // Parse URL to extract host and port
        let parsed_url = url::Url::parse(url)?;
        let host = parsed_url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid host"))?;
        let port = parsed_url.port().unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });

        // Open TCP connection
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).await?;

        // Send partial headers
        stream.write_all(partial_headers.as_bytes()).await?;
        stream.flush().await?;

        // Keep connection open but don't complete the request
        tokio::time::sleep(Duration::from_secs(300)).await; // Hold for 5 minutes

        Ok(())
    }

    /// Execute request with slow read (for slow read attack)
    pub async fn slow_read(&self, read_rate_bps: usize) -> Result<()> {
        let target = match &self.mode {
            ClientMode::SingleTarget { target } => target.clone(),
            ClientMode::MultiTarget { selector } => selector.select(),
        };

        let method = Method::from_bytes(target.method.as_bytes())
            .unwrap_or(Method::GET);

        let request = self.client
            .request(method, &target.url)
            .build()?;

        match self.client.execute(request).await {
            Ok(mut response) => {
                loop {
                    match response.chunk().await {
                        Ok(Some(_chunk)) => {
                            // Simulate slow reading
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Ok(None) => break, // End of stream
                        Err(_) => break,
                    }
                }
            }
            Err(_) => {}
        }

        Ok(())
    }
}
