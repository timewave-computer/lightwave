use serde_json::Value;

pub async fn get_execution_block_height_from_slot(
    beacon_node_url: &str,
    slot: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    let url = format!("{}/eth/v2/beacon/blocks/{}", beacon_node_url, slot);
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?.error_for_status()?;
    let json: Value = res.json().await?;
    let block_number = json["data"]["message"]["body"]["execution_payload"]["block_number"]
        .as_str()
        .ok_or("Missing block_number")?;
    let block_number = block_number.parse::<u64>()?;
    Ok(block_number)
}
