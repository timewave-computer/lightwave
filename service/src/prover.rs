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
use std::env;
use std::process::Command;
use std::time::{Duration, Instant};
use tendermint_program_types::TendermintOutput;
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

const DEFAULT_TIMEOUT: u64 = 60;

/// Reads the MODE environment variable once at startup
pub static MODE: Lazy<String> =
    Lazy::new(|| env::var("CLIENT_BACKEND").unwrap_or_else(|_| "HELIOS".to_string()));

/// Cleans up any existing SP1 GPU containers
fn cleanup_gpu_containers() -> Result<()> {
    let output = Command::new("docker")
        .args(["rm", "-f", "sp1-gpu"])
        .output()
        .context("Failed to execute docker command")?;

    if !output.status.success() {
        println!(
            "Warning: Failed to remove container: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Runs the main service loop that generates and verifies proofs
pub async fn run_prover_loop(
    state_manager: StateManager,
    mut service_state: ServiceState,
    recursive_elf: Vec<u8>,
    wrapper_elf: Vec<u8>,
    consensus_url: String,
) -> Result<()> {
    let start_time = Instant::now();
    loop {
        cleanup_gpu_containers()?;
        let client = ProverClient::from_env();
        let helios_elf = HELIOS_ELF.to_vec();
        let recursive_elf_clone = recursive_elf.clone();
        let wrapper_elf_clone = wrapper_elf.clone();

        let (recursive_pk, recursive_vk) = client.setup(&recursive_elf_clone);
        let (wrapper_pk, wrapper_vk) = client.setup(&wrapper_elf_clone);
        let _ = client.setup(&helios_elf);

        println!("[Prover Loop] Recursive VK: {:?}", recursive_vk.bytes32());
        println!("[Prover Loop] Wrapper VK: {:?}", wrapper_vk.bytes32());

        println!("[Prover Loop] Step 1/7");
        let recursive_prover = match MODE.as_str() {
            "HELIOS" => {
                helios_prover(
                    &helios_elf,
                    recursive_vk.bytes32(),
                    &service_state,
                    &consensus_url,
                )
                .await?
            }
            "TENDERMINT" => tendermint_prover(&service_state, recursive_vk.bytes32()).await?,
            _ => panic!("Invalid mode: {:?}", MODE.as_str()),
        };
        println!("[Prover Loop] Running recursive prover");

        println!("[Prover Loop] Step 2/7");
        let mut stdin = SP1Stdin::new();
        match recursive_prover.clone() {
            RecursiveProver::Helios((_, recursion_inputs)) => {
                stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            }
            RecursiveProver::Tendermint((_, recursion_inputs)) => {
                stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());
            }
        }

        println!("[Prover Loop] Step 3/7");
        // Run recursive proof in isolated task
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
                    println!("[Handled Error] Recursive proof failed: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
                Err(join_error) => {
                    println!("[PANIC] Recursive proof task panicked: {:?}", join_error);
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
            }
        };

        println!("[Prover Loop] Step 4/7");
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

        println!("[Prover Loop] Step 5/7");
        // Run wrapper proof in isolated task
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

            match handle.await {
                Ok(Ok(proof)) => proof,
                Ok(Err(e)) => {
                    println!("[Handled Error] Wrapper proof failed: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
                Err(join_error) => {
                    println!("[PANIC] Wrapper proof task panicked: {:?}", join_error);
                    tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                    continue;
                }
            }
        };

        println!("[Prover Loop] Step 6/7");
        match recursive_prover {
            RecursiveProver::Helios((helios_outputs, _)) => {
                let wrapped_outputs: HeliosRecursionCircuitOutputs =
                    borsh::from_slice(&recursive_proof.public_values.to_vec())
                        .expect("Failed to decode Helios outputs");
                service_state.most_recent_recursive_proof = Some(recursive_proof.clone());
                service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
                service_state.trusted_slot = helios_outputs.newHead.try_into().unwrap();
                service_state.trusted_height = wrapped_outputs.height;
                service_state.trusted_root = wrapped_outputs.root.try_into().unwrap();
                service_state.update_counter += 1;
            }
            RecursiveProver::Tendermint((tendermint_outputs, _)) => {
                let wrapped_outputs: TendermintRecursionCircuitOutputs =
                    borsh::from_slice(&recursive_proof.public_values.to_vec())
                        .expect("Failed to decode Tendermint outputs");
                service_state.most_recent_recursive_proof = Some(recursive_proof.clone());
                service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
                // in the case of tendermint, the trusted slot is the target height
                service_state.trusted_slot = tendermint_outputs.target_height.try_into().unwrap();
                service_state.trusted_height = wrapped_outputs.height;
                service_state.trusted_root = wrapped_outputs.root.try_into().unwrap();
                service_state.update_counter += 1;
            }
        }

        println!("[Prover Loop] Step 7/7");
        state_manager.save_state(&service_state)?;
        println!("New Service State: {:?} \n", service_state);
        println!("Alive for: {:?}", start_time.elapsed());
    }
}

async fn tendermint_prover(
    service_state: &ServiceState,
    recursive_vk: String,
) -> Result<RecursiveProver> {
    // Generate Helios proof in isolated task
    println!("[Tendermint] Step 1/2");
    let tendermint_proof = {
        cleanup_gpu_containers()?;
        let tendermint_rpc_client = TendermintRPCClient::default();
        let tendermint_height = tendermint_rpc_client.get_latest_block_height().await;
        let tendermint_prover = TendermintProver::new();
        let (trusted_light_block, target_light_block) = tendermint_rpc_client
            .get_light_blocks(service_state.trusted_height, tendermint_height)
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

    println!("[Tendermint] Step 2/2");
    let tendermint_outputs: TendermintOutput =
        serde_json::from_slice(&tendermint_proof.public_values.to_vec()).unwrap();

    let previous_proof = service_state.most_recent_recursive_proof.clone();

    let recursion_inputs = TendermintRecursionCircuitInputs {
        tendermint_proof: tendermint_proof.bytes(),
        tendermint_public_values: tendermint_proof.public_values.to_vec(),
        recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
        recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
        recursive_vk: recursive_vk,
        trusted_height: service_state.trusted_height,
    };

    Ok(RecursiveProver::Tendermint((
        tendermint_outputs,
        recursion_inputs,
    )))
}

async fn helios_prover(
    helios_elf: &[u8],
    recursive_vk: String,
    service_state: &ServiceState,
    consensus_url: &str,
) -> Result<RecursiveProver> {
    // Generate Helios proof in isolated task
    println!("[Helios] Step 1/4");
    let preprocessor = Preprocessor::new(service_state.trusted_slot);
    let inputs = match preprocessor.run().await {
        Ok(inputs) => inputs,
        Err(e) => {
            return Err(anyhow::anyhow!("Helios proof task panicked: {:?}", e));
        }
    };

    println!("[Helios] Step 2/4");
    let mut stdin = SP1Stdin::new();
    stdin.write_slice(&inputs);
    let helios_proof = {
        let stdin_clone = stdin.clone();
        cleanup_gpu_containers()?;
        let client = ProverClient::from_env();
        let (helios_pk, _) = client.setup(&helios_elf);

        let handle =
            tokio::spawn(async move { client.prove(&helios_pk, &stdin_clone).groth16().run() });

        match handle.await {
            Ok(Ok(proof)) => proof,
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Helios proof task panicked: {:?}", e));
            }
            Err(join_error) => {
                return Err(anyhow::anyhow!(
                    "Helios proof task panicked: {:?}",
                    join_error
                ));
            }
        }
    };

    println!("[Helios] Step 3/4");
    let helios_outputs: HeliosOutputs =
        HeliosOutputs::abi_decode(&helios_proof.public_values.to_vec(), false).unwrap();

    let electra_block = get_electra_block(helios_outputs.newHead.try_into()?, &consensus_url).await;
    let electra_body_roots = extract_electra_block_body(electra_block);
    let beacon_header =
        get_beacon_block_header(helios_outputs.newHead.try_into()?, &consensus_url).await;

    let electra_header = ElectraBlockHeader {
        slot: beacon_header.slot.as_u64(),
        proposer_index: beacon_header.proposer_index,
        parent_root: beacon_header.parent_root.to_vec().try_into().unwrap(),
        state_root: beacon_header.state_root.to_vec().try_into().unwrap(),
        body_root: beacon_header.body_root.to_vec().try_into().unwrap(),
    };

    println!("[Helios] Step 4/4");
    let previous_proof = service_state.most_recent_recursive_proof.clone();

    let recursion_inputs = HeliosRecursionCircuitInputs {
        electra_body_roots,
        electra_header,
        helios_proof: helios_proof.bytes(),
        helios_public_values: helios_proof.public_values.to_vec(),
        recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
        recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
        recursive_vk: recursive_vk,
        previous_head: service_state.trusted_slot,
    };

    Ok(RecursiveProver::Helios((helios_outputs, recursion_inputs)))
}

#[derive(Clone)]
enum RecursiveProver {
    Helios((HeliosOutputs, HeliosRecursionCircuitInputs)),
    Tendermint((TendermintOutput, TendermintRecursionCircuitInputs)),
}
