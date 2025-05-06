use core::fmt;
use std::fmt::Debug;

use alloy::dyn_abi::SolType;
use anyhow::{Context, Result};
use preprocessor::Preprocessor;
use sp1_helios_primitives::types::{ProofInputs as HeliosInputs, ProofOutputs as HeliosOutputs};
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1Stdin};
mod preprocessor;
use tree_hash::TreeHash;
pub const ELF: &[u8] = include_bytes!("../../elf/sp1-helios-elf");

pub struct ServiceState {
    pub genesis_committee_hash: Option<String>,
    pub most_recent_proof: Option<SP1ProofWithPublicValues>,
    pub trusted_slot: u64,
}

impl Debug for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceState")
            .field("genesis_committee_hash", &self.genesis_committee_hash)
            .field(
                "most_recent_proof_outputs",
                &self.most_recent_proof.as_ref().map(|proof| {
                    HeliosOutputs::abi_decode(&proof.public_values.to_vec(), false)
                        .map(|outputs| format!("{:?}", outputs))
                        .unwrap_or_default()
                }),
            )
            .field("trusted_slot", &self.trusted_slot)
            .finish()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut service_state = ServiceState {
        genesis_committee_hash: None,
        most_recent_proof: None,
        trusted_slot: 7561216 - (32 * 254),
    };
    dotenvy::dotenv().ok();
    let client = ProverClient::from_env();
    let (pk, _) = client.setup(ELF);
    let mut iterations = 0;
    loop {
        let mut stdin = SP1Stdin::new();
        let preprocessor = Preprocessor::new(service_state.trusted_slot);
        let inputs = match preprocessor.run().await {
            Ok(inputs) => inputs,
            Err(e) => {
                println!("[Warning]: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
        };
        // if this is the first proof, we want to store the active committee as our genesis committee
        if iterations == 0 {
            let helios_inputs: HeliosInputs = serde_cbor::from_slice(&inputs)?;
            service_state.genesis_committee_hash = Some(hex::encode(
                helios_inputs
                    .store
                    .current_sync_committee
                    .clone()
                    .tree_hash_root()
                    .to_vec(),
            ));
        }
        stdin.write_slice(&inputs);
        let proof = match client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .context("Failed to prove")
        {
            Ok(proof) => proof,
            Err(e) => {
                println!("Proof failed with error: {:?}", e);
                continue;
            }
        };
        service_state.most_recent_proof = Some(proof.clone());
        let helios_output: HeliosOutputs =
            HeliosOutputs::abi_decode(&proof.public_values.to_vec(), false).unwrap();
        service_state.trusted_slot = helios_output.newHead.try_into().unwrap();
        println!("New Service State: {:?} \n", service_state);
        iterations += 1;
        if iterations >= 3 {
            return Ok(());
        }
    }
}
