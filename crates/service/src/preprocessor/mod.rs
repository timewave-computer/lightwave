use anyhow::{Context, Result};
use helios_ethereum::rpc::ConsensusRpc;
use serde_json::Value;
use sp1_helios_primitives::types::ProofInputs;
use std::env;
use tracing::info;

use crate::preprocessor::helios::{get_checkpoint, get_client, get_updates};
mod helios;
mod helpers;

/// Type alias for the serialized Helios program inputs
pub type HeliosInputSlice = Vec<u8>;

/// Preprocessor responsible for preparing inputs for the Helios light client program.
///
/// The preprocessor:
/// 1. Takes a trusted slot as input
/// 2. Fetches the latest finalized slot from the consensus layer
/// 3. Calculates the period distance between slots
/// 4. Gathers necessary updates and finality data
/// 5. Serializes all inputs for the Helios program
pub struct Preprocessor {
    /// The trusted slot to use as a reference point
    pub trusted_slot: u64,
}

impl Preprocessor {
    /// Creates a new Preprocessor instance with the given trusted slot
    pub fn new(trusted_slot: u64) -> Self {
        Self { trusted_slot }
    }

    /// Runs the preprocessing pipeline to generate inputs for the Helios program.
    ///
    /// This function:
    /// 1. Gets the checkpoint for the trusted slot
    /// 2. Initializes the Helios client
    /// 3. Calculates period distances
    /// 4. Fetches updates and finality data
    /// 5. Serializes everything into the format expected by the Helios program
    pub async fn run(&self) -> Result<HeliosInputSlice> {
        let checkpoint = get_checkpoint(self.trusted_slot).await?;
        let client = get_client(checkpoint).await?;
        let trusted_slot_period = &self.trusted_slot / 8192;
        let latest_slot = gest_latest_slot().await?;
        // we only get a finality update every 32 slots, so we need to wait for the
        // latest finalized slot to be at least 32 slots ahead of the trusted slot
        if latest_slot <= self.trusted_slot || latest_slot / 32 < self.trusted_slot / 32 {
            return Err(anyhow::anyhow!(
                "Waiting for new slot to be finalized, retry in 60 seconds!"
            ));
        }

        let latest_finalized_slot = latest_slot - (latest_slot % 32);
        info!(
            "latest_finalized_slot: {}, trusted_slot: {}",
            latest_finalized_slot, self.trusted_slot
        );
        let latest_finalized_slot_period = latest_finalized_slot / 8192;
        let mut period_distance = latest_finalized_slot_period - trusted_slot_period;
        if period_distance == 0 {
            // minimum period distance is 1
            period_distance = 1;
        }
        let updates = get_updates(&client, period_distance as u8)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get updates: {}", e))?;
        let finality_update = client
            .rpc
            .get_finality_update()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get finality update: {}", e))?;
        // Create program inputs
        let expected_current_slot = client.expected_current_slot();
        let inputs = ProofInputs {
            updates,
            finality_update,
            expected_current_slot,
            store: client.store.clone(),
            genesis_root: client.config.chain.genesis_root,
            forks: client.config.forks.clone(),
        };
        serde_cbor::to_vec(&inputs).context("Failed to serialize proof inputs")
    }
}

/// Fetches the latest finalized slot from the consensus layer.
///
/// This function makes an RPC call to the consensus client to get
/// the most recently finalized slot number.
pub async fn gest_latest_slot() -> Result<u64> {
    let consensus_url = env::var("SOURCE_CONSENSUS_RPC_URL")?;
    let resp: Value = reqwest::get(format!("{}/eth/v1/beacon/headers/finalized", consensus_url))
        .await?
        .json()
        .await?;

    let slot_str = resp["data"]["header"]["message"]["slot"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get slot from response!"))?;

    let slot = slot_str.parse::<u64>()?;
    Ok(slot)
}
