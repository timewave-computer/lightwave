use core::fmt;
use std::{fmt::Debug, time::Instant};

use alloy_sol_types::SolValue;
use anyhow::{Context, Result};
use preprocessor::Preprocessor;
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
use sp1_helios_primitives::types::{ProofInputs as HeliosInputs, ProofOutputs as HeliosOutputs};
use sp1_sdk::{HashableKey, ProverClient, SP1ProofWithPublicValues, SP1Stdin, include_elf};
mod preprocessor;
use tree_hash::TreeHash;
pub const HELIOS_ELF: &[u8] = include_bytes!("../../elf/sp1-helios-elf");
pub const RECURSIVE_ELF: &[u8] = include_elf!("recursion-circuit");

pub struct ServiceState {
    // we can remove this, it's just to print the genesis committee hash for convenience
    pub genesis_committee_hash: Option<String>,
    // our last recursive proof
    pub most_recent_proof: Option<SP1ProofWithPublicValues>,
    // the last trusted slot from our recursive proof outputs
    pub trusted_slot: u64,
    // the current root
    pub current_root: [u8; 32],
    // increases with every recursive proof
    pub height: u64,
}

impl Debug for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceState")
            .field("genesis_committee_hash", &self.genesis_committee_hash)
            .field(
                "most_recent_proof_outputs",
                &self.most_recent_proof.as_ref().map(|proof| {
                    borsh::from_slice::<RecursionCircuitOutputs>(&proof.public_values.to_vec())
                        .map(|outputs| format!("{:?}", outputs))
                        .unwrap_or_default()
                }),
            )
            .field("trusted_slot", &self.trusted_slot)
            .field("current_root", &hex::encode(self.current_root))
            .field("height", &self.height)
            .finish()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Instant::now();
    let mut service_state = ServiceState {
        genesis_committee_hash: None,
        most_recent_proof: None,
        trusted_slot: 7553088,
        current_root: [0; 32],
        height: 0,
    };
    dotenvy::dotenv().ok();
    let client = ProverClient::from_env();
    loop {
        let (helios_pk, helios_vk) = client.setup(HELIOS_ELF);
        let (recursive_pk, recursive_vk) = client.setup(RECURSIVE_ELF);
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
        if service_state.height == 0 {
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
            .prove(&helios_pk, &stdin)
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
        // generate the recursive proof
        if service_state.height == 0 {
            let recursion_inputs = RecursionCircuitInputs {
                helios_proof: proof.bytes(),
                helios_public_values: proof.public_values.to_vec(),
                helios_vk: helios_vk.bytes32(),
                previous_head: service_state.trusted_slot,
                previous_proof: None,
                previous_public_values: None,
                previous_vk: None,
            };
            let mut stdin = SP1Stdin::new();
            stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            let recursive_proof = client
                .prove(&recursive_pk, &stdin)
                .groth16()
                .run()
                .context("Failed to prove")?;
            service_state.most_recent_proof = Some(recursive_proof.clone());
            let recursive_outputs: RecursionCircuitOutputs =
                borsh::from_slice(&recursive_proof.public_values.to_vec()).unwrap();
            service_state.trusted_slot = recursive_outputs.slot;
            service_state.current_root = recursive_outputs.root.try_into().unwrap();
        } else {
            let previous_proof = service_state
                .most_recent_proof
                .expect("Missing previous proof in state");
            let recursion_inputs = RecursionCircuitInputs {
                helios_proof: proof.bytes(),
                helios_public_values: proof.public_values.to_vec(),
                helios_vk: helios_vk.bytes32(),
                previous_head: service_state.trusted_slot,
                previous_proof: Some(previous_proof.bytes()),
                previous_public_values: Some(previous_proof.public_values.to_vec()),
                previous_vk: Some(recursive_vk.bytes32()),
            };
            let mut stdin = SP1Stdin::new();
            stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            let recursive_proof = client
                .prove(&recursive_pk, &stdin)
                .groth16()
                .run()
                .context("Failed to prove")?;
            let recursive_outputs: RecursionCircuitOutputs =
                borsh::from_slice(&recursive_proof.public_values.to_vec()).unwrap();
            service_state.most_recent_proof = Some(recursive_proof.clone());
            service_state.trusted_slot = recursive_outputs.slot;
            service_state.current_root = recursive_outputs.root.try_into().unwrap();
        }
        println!("New Service State: {:?} \n", service_state);
        let elapsed_time = start_time.elapsed();
        println!("Alive for: {:?}", elapsed_time);
        service_state.height += 1;
    }
}
