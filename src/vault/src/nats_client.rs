use anyhow::Result;
use async_nats::Client;

pub async fn connect_nats(url: &str) -> Result<Client> {
    let client = async_nats::connect(url).await?;
    Ok(client)
}
