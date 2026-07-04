use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    http_traffic_sim::run().await
}