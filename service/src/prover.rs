use alloy_sol_types::SolType;
use anyhow::{Context, Result};
use beacon_electra::{
    extract_electra_block_body, get_beacon_block_header, get_electra_block,
    types::electra::ElectraBlockHeader,
};
use helios_recursion_types::{
    RecursionCircuitInputs as HeliosRecursionCircuitInputs,
    RecursionCircuitOutputs as HeliosRecursionCircuitOutputs,
    WrapperCircuitInputs as HeliosWrapperCircuitInputs,
};
use once_cell::sync::Lazy;
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use sp1_tendermint_primitives::TendermintOutput;
use std::cmp::min;
use std::env;
use std::process::Command;
use std::time::{Duration, Instant};
use tendermint_prover::TendermintProver;
use tendermint_prover::util::TendermintRPCClient;
use tendermint_recursion_types::{
    RecursionCircuitInputs as TendermintRecursionCircuitInputs,
    RecursionCircuitOutputs as TendermintRecursionCircuitOutputs,
    WrapperCircuitInputs as TendermintWrapperCircuitInputs,
};

use crate::{
    HELIOS_ELF,
    preprocessor::Preprocessor,
    state::{ServiceState, StateManager},
};

/// Default timeout in seconds for retry operations
const DEFAULT_TIMEOUT: u64 = 60;

/// Reads the MODE environment variable once at startup
/// Determines whether to use HELIOS or TENDERMINT consensus
pub static MODE: Lazy<String> =
    Lazy::new(|| env::var("CLIENT_BACKEND").unwrap_or_else(|_| "HELIOS".to_string()));

/// Cleans up any existing SP1 GPU containers to prevent conflicts
fn cleanup_gpu_containers() -> Result<()> {
    let output = Command::new("docker")
        .args(["rm", "-f", "sp1-gpu"])
        .output()
        .context("Failed to execute docker command")?;

    if !output.status.success() {
        tracing::info!(
            "Warning: Failed to remove container: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Runs the main service loop that generates and verifies proofs
///
/// This function orchestrates the entire proof generation process:
/// 1. Sets up prover clients and verification keys
/// 2. Generates base proofs (Helios or Tendermint)
/// 3. Generates recursive proofs
/// 4. Generates wrapper proofs
/// 5. Updates service state with new trusted information
/// 6. Saves state and continues the loop
pub async fn run_prover_loop(
    state_manager: StateManager,
    mut service_state: ServiceState,
    recursive_elf: Vec<u8>,
    wrapper_elf: Vec<u8>,
    consensus_url: String,
) -> Result<()> {
    let start_time = Instant::now();
    loop {
        // Clean up any existing GPU containers
        cleanup_gpu_containers()?;

        // Initialize prover client and load ELF files
        let client = ProverClient::from_env();
        let helios_elf = HELIOS_ELF.to_vec();
        let recursive_elf_clone = recursive_elf.clone();
        let wrapper_elf_clone = wrapper_elf.clone();

        // Set up verification keys for all circuits
        let (recursive_pk, recursive_vk) = client.setup(&recursive_elf_clone);
        let (wrapper_pk, wrapper_vk) = client.setup(&wrapper_elf_clone);
        let _ = client.setup(&helios_elf);

        tracing::info!(
            proof_type = "recursive",
            vk = %recursive_vk.bytes32(),
            "Proof verification key generated"
        );
        tracing::info!(
            proof_type = "wrapper",
            vk = %wrapper_vk.bytes32(),
            "Proof verification key generated"
        );

        // Generate base proof based on selected mode
        let recursive_prover = match MODE.as_str() {
            "HELIOS" => {
                match helios_prover(
                    &helios_elf,
                    recursive_vk.bytes32(),
                    &service_state,
                    &consensus_url,
                )
                .await
                {
                    Ok(prover) => prover,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            mode = "HELIOS",
                            "Prover failed, retrying"
                        );
                        tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                        continue;
                    }
                }
            }
            "TENDERMINT" => match tendermint_prover(&service_state, recursive_vk.bytes32()).await {
                Ok(prover) => prover,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        mode = "TENDERMINT",
                        "Prover failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
            },
            _ => panic!("Invalid mode: {:?}", MODE.as_str()),
        };

        // Prepare inputs for recursive proof generation
        let mut stdin = SP1Stdin::new();
        match recursive_prover.clone() {
            RecursiveProver::Helios((_, recursion_inputs)) => {
                stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            }
            RecursiveProver::Tendermint((_, recursion_inputs)) => {
                stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            }
        }

        tracing::info!(proof_type = "recursive", "Generating proof");
        // Run recursive proof generation in isolated task
        let recursive_proof = {
            let recursive_pk_clone = recursive_pk.clone();
            let stdin_clone = stdin.clone();
            cleanup_gpu_containers()?;
            let client = ProverClient::from_env();

            let _ = client.setup(&recursive_elf);

            let handle = tokio::spawn(async move {
                client
                    .prove(&recursive_pk_clone, &stdin_clone)
                    .groth16()
                    .run()
            });

            match handle.await {
                Ok(Ok(proof)) => proof,
                Ok(Err(e)) => {
                    tracing::error!(
                        error = %e,
                        proof_type = "recursive",
                        "Proof generation failed"
                    );
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
                Err(join_error) => {
                    tracing::error!(
                        error = %join_error,
                        proof_type = "recursive",
                        "Proof task failed"
                    );
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
            }
        };
        tracing::info!(proof_type = "recursive", "Proof generated successfully");

        // Prepare inputs for wrapper proof generation
        let mut stdin = SP1Stdin::new();
        match recursive_prover {
            RecursiveProver::Helios(_) => {
                let wrapper_inputs = HeliosWrapperCircuitInputs {
                    recursive_proof: recursive_proof.bytes(),
                    recursive_public_values: recursive_proof.public_values.to_vec(),
                };
                stdin.write_slice(&borsh::to_vec(&wrapper_inputs).unwrap());
            }
            RecursiveProver::Tendermint(_) => {
                let wrapper_inputs = TendermintWrapperCircuitInputs {
                    recursive_proof: recursive_proof.bytes(),
                    recursive_public_values: recursive_proof.public_values.to_vec(),
                };
                stdin.write_slice(&borsh::to_vec(&wrapper_inputs).unwrap());
            }
        }

        tracing::info!(proof_type = "wrapper", "Generating proof");
        // Run wrapper proof generation in isolated task
        let final_wrapped_proof = {
            let wrapper_pk_clone = wrapper_pk.clone();
            let stdin_clone = stdin.clone();
            cleanup_gpu_containers()?;
            let client = ProverClient::from_env();

            let handle = tokio::spawn(async move {
                let _ = client.setup(&wrapper_elf_clone);
                client
                    .prove(&wrapper_pk_clone, &stdin_clone)
                    .groth16()
                    .run()
            });
            tracing::info!(proof_type = "wrapper", "Proof generated successfully");

            match handle.await {
                Ok(Ok(proof)) => proof,
                Ok(Err(e)) => {
                    tracing::error!(
                        error = %e,
                        proof_type = "wrapper",
                        "Proof generation failed"
                    );
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
                Err(join_error) => {
                    tracing::error!(
                        error = %join_error,
                        proof_type = "wrapper",
                        "Proof task failed"
                    );
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
            }
        };

        // Update service state with new trusted information
        match recursive_prover {
            RecursiveProver::Helios((helios_outputs, _)) => {
                let wrapped_outputs: HeliosRecursionCircuitOutputs =
                    borsh::from_slice(&recursive_proof.public_values.to_vec())
                        .expect("Failed to decode Helios outputs");
                service_state.most_recent_recursive_proof = Some(recursive_proof.clone());
                service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
                service_state.trusted_slot = helios_outputs.newHead.try_into().unwrap();
                service_state.trusted_height = wrapped_outputs.height;
                service_state.trusted_root = wrapped_outputs.root;
                service_state.update_counter += 1;
            }
            RecursiveProver::Tendermint((tendermint_outputs, _)) => {
                let wrapped_outputs: TendermintRecursionCircuitOutputs =
                    borsh::from_slice(&recursive_proof.public_values.to_vec())
                        .expect("Failed to decode Tendermint outputs");
                service_state.most_recent_recursive_proof = Some(recursive_proof.clone());
                service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
                // In the case of Tendermint, the trusted slot is the target height
                service_state.trusted_slot = tendermint_outputs.target_height;
                service_state.trusted_height = wrapped_outputs.height;
                service_state.trusted_root = wrapped_outputs.root;
                service_state.update_counter += 1;
            }
        }

        // Save updated state to persistent storage
        tracing::info!("Saving service state");
        state_manager.save_state(&service_state)?;
        tracing::info!(
            root = %format!("{:?}", service_state.trusted_root),
            slot = service_state.trusted_slot,
            height = service_state.trusted_height,
            "Service state updated"
        );
        tracing::info!(
            uptime = ?start_time.elapsed(),
            "Service status"
        );
    }
}

/// Generates a Tendermint proof and prepares recursive circuit inputs
///
/// This function:
/// 1. Connects to Tendermint RPC to get latest block information
/// 2. Generates a Tendermint proof for the target block range
/// 3. Prepares inputs for the recursive circuit
async fn tendermint_prover(
    service_state: &ServiceState,
    recursive_vk: String,
) -> Result<RecursiveProver> {
    dotenvy::dotenv().ok();

    tracing::info!("Generating Tendermint proof");
    let tendermint_proof = {
        cleanup_gpu_containers()?;

        // Get expiration limit from environment
        let tendermint_expiration_limit = std::env::var("TENDERMINT_EXPIRATION_LIMIT")
            .unwrap_or_else(|_| "100000".to_string())
            .parse::<u64>()
            .unwrap_or(100_000);

        let tendermint_rpc_client = TendermintRPCClient::default();
        let tendermint_height = tendermint_rpc_client.get_latest_block_height().await;
        let tendermint_prover = TendermintProver::new();

        // Calculate target height with expiration limit
        let target_height = min(
            tendermint_height,
            service_state.trusted_height + tendermint_expiration_limit,
        );

        // Get light blocks for proof generation
        let (trusted_light_block, target_light_block) = tendermint_rpc_client
            .get_light_blocks(service_state.trusted_height, target_height)
            .await;

        let handle = tokio::spawn(async move {
            tendermint_prover.generate_tendermint_proof(&trusted_light_block, &target_light_block)
        });

        match handle.await {
            Ok(proof) => proof,
            Err(join_error) => {
                return Err(anyhow::anyhow!(
                    "Tendermint proof task panicked: {:?}",
                    join_error
                ));
            }
        }
    };
    tracing::info!("Tendermint proof generated");

    // Decode proof outputs
    let tendermint_outputs: TendermintOutput =
        serde_json::from_slice(&tendermint_proof.public_values.to_vec()).unwrap();

    let previous_proof = service_state.most_recent_recursive_proof.clone();

    // Prepare recursive circuit inputs
    let recursion_inputs = TendermintRecursionCircuitInputs {
        tendermint_proof: tendermint_proof.bytes(),
        tendermint_public_values: tendermint_proof.public_values.to_vec(),
        recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
        recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
        recursive_vk,
        trusted_height: service_state.trusted_height,
    };

    Ok(RecursiveProver::Tendermint((
        tendermint_outputs,
        recursion_inputs,
    )))
}

/// Generates a Helios proof and prepares recursive circuit inputs
///
/// This function:
/// 1. Runs the Helios preprocessor to get block data
/// 2. Generates a Helios proof for the target slot
/// 3. Fetches Electra block information from consensus layer
/// 4. Prepares inputs for the recursive circuit
async fn helios_prover(
    helios_elf: &[u8],
    recursive_vk: String,
    service_state: &ServiceState,
    consensus_url: &str,
) -> Result<RecursiveProver> {
    // Run Helios preprocessor to get block inputs
    tracing::info!("Running Helios preprocessor");
    let preprocessor = Preprocessor::new(service_state.trusted_slot);
    let inputs = match preprocessor.run().await {
        Ok(inputs) => inputs,
        Err(e) => {
            return Err(anyhow::anyhow!("Helios preprocessor failed: {:?}", e));
        }
    };
    tracing::info!("Helios preprocessor concluded");

    // Prepare inputs for Helios proof generation
    let mut stdin = SP1Stdin::new();
    stdin.write_slice(&inputs);

    tracing::info!("Generating Helios proof");
    let helios_proof = {
        let stdin_clone = stdin.clone();
        cleanup_gpu_containers()?;
        let client = ProverClient::from_env();
        let (helios_pk, _) = client.setup(helios_elf);

        let handle =
            tokio::spawn(async move { client.prove(&helios_pk, &stdin_clone).groth16().run() });

        match handle.await {
            Ok(Ok(proof)) => proof,
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("{:?}", e));
            }
            Err(join_error) => {
                return Err(anyhow::anyhow!("{:?}", join_error));
            }
        }
    };
    tracing::info!("Helios proof generated");

    // Decode proof outputs
    let helios_outputs: HeliosOutputs =
        HeliosOutputs::abi_decode(&helios_proof.public_values.to_vec(), false).unwrap();

    // Fetch Electra block information from consensus layer
    tracing::info!("Getting electra block");
    let electra_block = get_electra_block(helios_outputs.newHead.try_into()?, consensus_url).await;
    let electra_body_roots = extract_electra_block_body(electra_block);
    let beacon_header =
        get_beacon_block_header(helios_outputs.newHead.try_into()?, consensus_url).await;
    tracing::info!("Electra block retrieved");

    // Create Electra block header
    let electra_header = ElectraBlockHeader {
        slot: beacon_header.slot.as_u64(),
        proposer_index: beacon_header.proposer_index,
        parent_root: beacon_header.parent_root.to_vec().try_into().unwrap(),
        state_root: beacon_header.state_root.to_vec().try_into().unwrap(),
        body_root: beacon_header.body_root.to_vec().try_into().unwrap(),
    };

    let previous_proof = service_state.most_recent_recursive_proof.clone();

    // Prepare recursive circuit inputs
    let recursion_inputs = HeliosRecursionCircuitInputs {
        electra_body_roots,
        electra_header,
        helios_proof: helios_proof.bytes(),
        helios_public_values: helios_proof.public_values.to_vec(),
        recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
        recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
        recursive_vk,
        previous_head: service_state.trusted_slot,
    };

    Ok(RecursiveProver::Helios((helios_outputs, recursion_inputs)))
}

/// Enum representing different types of recursive provers
///
/// This allows the main loop to handle both Helios and Tendermint
/// consensus mechanisms with a unified interface.
#[derive(Clone)]
enum RecursiveProver {
    Helios((HeliosOutputs, HeliosRecursionCircuitInputs)),
    Tendermint((TendermintOutput, TendermintRecursionCircuitInputs)),
}
