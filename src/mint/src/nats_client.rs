use async_nats::Client;
use anyhow::Result;

pub async fn connect(nats_url: &str) -> Result<Client> {
    let client = async_nats::connect(nats_url).await?;
    Ok(client)
}