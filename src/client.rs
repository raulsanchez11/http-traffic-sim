//! HTTP client module for executing requests and stress testing patterns.

use anyhow::{Context, Result};
use reqwest::{Client, Method, Request};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::{ClientConfig, TargetConfig};
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

fn build_reqwest_client(client_config: &ClientConfig) -> Result<Client> {
    let timeout = Duration::from_secs(client_config.timeout_secs);
    let mut builder = Client::builder()
        .timeout(timeout)
        .pool_max_idle_per_host(client_config.pool_max_idle_per_host)
        .tcp_keepalive(Some(Duration::from_secs(60)));

    if client_config.http2_prior_knowledge {
        builder = builder.http2_prior_knowledge();
    }

    builder.build().map_err(Into::into)
}

impl HttpClient {
    pub fn new(target: TargetConfig, client_config: &ClientConfig) -> Result<Self> {
        Ok(Self {
            client: build_reqwest_client(client_config)?,
            mode: ClientMode::SingleTarget {
                target: Arc::new(target),
            },
        })
    }

    pub fn new_multi_target(
        selector: Arc<TargetSelector>,
        client_config: &ClientConfig,
    ) -> Result<Self> {
        Ok(Self {
            client: build_reqwest_client(client_config)?,
            mode: ClientMode::MultiTarget { selector },
        })
    }

    pub fn selected_target(&self) -> Arc<TargetConfig> {
        match &self.mode {
            ClientMode::SingleTarget { target } => target.clone(),
            ClientMode::MultiTarget { selector } => selector.select(),
        }
    }

    pub async fn execute(&self) -> RequestResult {
        let start = Instant::now();
        let target = self.selected_target();
        let method = Method::from_bytes(target.method.as_bytes()).unwrap_or(Method::GET);

        let mut request_builder = self.client.request(method, &target.url);
        for (key, value) in &target.headers {
            request_builder = request_builder.header(key, value);
        }
        if let Some(body) = &target.body {
            request_builder = request_builder.body(body.clone());
        }

        match request_builder.build() {
            Ok(request) => self.send_and_measure(request, start, &target.id).await,
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(format!("Failed to build request: {}", e)),
                target_id: target.id.clone(),
            },
        }
    }

    async fn send_and_measure(
        &self,
        request: Request,
        start: Instant,
        target_id: &str,
    ) -> RequestResult {
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

    pub async fn execute_and_hold(&self, hold_duration: Duration) -> RequestResult {
        let start = Instant::now();
        let target = self.selected_target();

        match self
            .hold_raw_connection(&target.url, hold_duration)
            .await
        {
            Ok(()) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: true,
                error: None,
                target_id: target.id.clone(),
            },
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(e.to_string()),
                target_id: target.id.clone(),
            },
        }
    }

    pub async fn send_partial_request(&self, url: &str, partial_headers: &str) -> Result<()> {
        let (host, port) = parse_host_port(url)?;
        let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
        stream.write_all(partial_headers.as_bytes()).await?;
        stream.flush().await?;
        tokio::time::sleep(Duration::from_secs(300)).await;
        Ok(())
    }

    pub async fn execute_slow_post(
        &self,
        bytes_per_second: usize,
        payload_size: usize,
    ) -> RequestResult {
        let start = Instant::now();
        let target = self.selected_target();
        let rate = bytes_per_second.max(1);
        let delay = Duration::from_secs_f64(1.0 / rate as f64);

        let result = async {
            let (host, port) = parse_host_port(&target.url)?;
            let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
            let path = url::Url::parse(&target.url)?.path().to_string();
            let headers = format!(
                "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Length: {payload_size}\r\nContent-Type: application/octet-stream\r\n\r\n"
            );
            stream.write_all(headers.as_bytes()).await?;

            let mut sent = 0usize;
            while sent < payload_size {
                stream.write_all(b"x").await?;
                sent += 1;
                tokio::time::sleep(delay).await;
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;

        RequestResult {
            start_time: start,
            duration: start.elapsed(),
            status_code: None,
            success: result.is_ok(),
            error: result.err().map(|e| e.to_string()),
            target_id: target.id.clone(),
        }
    }

    pub async fn execute_large_payload(&self, size_mb: usize) -> RequestResult {
        let start = Instant::now();
        let target = self.selected_target();
        let payload = vec![b'x'; size_mb.saturating_mul(1024 * 1024).max(1)];

        let mut request_builder = self
            .client
            .request(Method::POST, &target.url)
            .body(payload);
        for (key, value) in &target.headers {
            request_builder = request_builder.header(key, value);
        }

        match request_builder.build() {
            Ok(request) => self.send_and_measure(request, start, &target.id).await,
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(format!("Failed to build large payload request: {}", e)),
                target_id: target.id.clone(),
            },
        }
    }

    pub async fn execute_pipelined(&self, requests_per_connection: usize) -> RequestResult {
        let start = Instant::now();
        let target = self.selected_target();

        let result = async {
            let (host, port) = parse_host_port(&target.url)?;
            let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
            let path = url::Url::parse(&target.url)?.path().to_string();
            let mut pipeline = String::new();
            for _ in 0..requests_per_connection.max(1) {
                pipeline.push_str(&format!(
                    "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: keep-alive\r\n\r\n"
                ));
            }
            stream.write_all(pipeline.as_bytes()).await?;
            stream.flush().await?;
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        RequestResult {
            start_time: start,
            duration: start.elapsed(),
            status_code: None,
            success: result.is_ok(),
            error: result.err().map(|e| e.to_string()),
            target_id: target.id.clone(),
        }
    }

    pub async fn slow_read(&self, read_rate_bps: usize) -> Result<()> {
        let target = self.selected_target();
        let method = Method::from_bytes(target.method.as_bytes()).unwrap_or(Method::GET);
        let request = self.client.request(method, &target.url).build()?;
        let rate = read_rate_bps.max(1);
        let delay = Duration::from_secs_f64(8.0 / rate as f64).max(Duration::from_millis(1));

        if let Ok(mut response) = self.client.execute(request).await {
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let chunk_delay =
                            Duration::from_secs_f64(chunk.len() as f64 / rate as f64)
                                .max(delay);
                        tokio::time::sleep(chunk_delay).await;
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }

        Ok(())
    }

    async fn hold_raw_connection(&self, url: &str, hold_duration: Duration) -> Result<()> {
        let (host, port) = parse_host_port(url)?;
        let parsed = url::Url::parse(url)?;
        let path = if parsed.path().is_empty() {
            "/"
        } else {
            parsed.path()
        };

        let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
        let partial = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n");
        stream.write_all(partial.as_bytes()).await?;
        stream.flush().await?;
        tokio::time::sleep(hold_duration).await;
        Ok(())
    }
}

// Note: some duplication remains in raw TCP URL parsing for stress attack methods
// (see execute_slow_post, execute_pipelined, etc.). parse_host_port is shared but
// full request construction logic is bespoke per attack type.
fn parse_host_port(url: &str) -> Result<(String, u16)> {
    let parsed = url::Url::parse(url).with_context(|| format!("Invalid URL: {url}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL has no host: {url}"))?
        .to_string();
    let port = parsed.port().unwrap_or_else(|| {
        if parsed.scheme() == "https" {
            443
        } else {
            80
        }
    });
    Ok((host, port))
}