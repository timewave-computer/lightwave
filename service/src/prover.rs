use alloy_sol_types::SolType;
use anyhow::{Context, Result};
use beacon_electra::{
    extract_electra_block_body, get_beacon_block_header, get_electra_block,
    types::electra::ElectraBlockHeader,
};
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs, WrapperCircuitInputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::time::{Duration, Instant};

use crate::{
    HELIOS_ELF,
    preprocessor::Preprocessor,
    state::{ServiceState, StateManager},
};

const DEFAULT_TIMEOUT: u64 = 60;

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
        let client = ProverClient::from_env();
        let helios_elf = HELIOS_ELF.to_vec();
        let recursive_elf_clone = recursive_elf.clone();
        let wrapper_elf_clone = wrapper_elf.clone();

        // Setup all circuits once at the start
        let (helios_pk, _) = client.setup(&helios_elf);
        let (recursive_pk, recursive_vk) = client.setup(&recursive_elf_clone);
        let (wrapper_pk, _) = client.setup(&wrapper_elf_clone);

        let _ = client.setup(&helios_elf);

        println!("[Debug] Recursive VK: {:?}", recursive_vk.bytes32());

        let preprocessor = Preprocessor::new(service_state.trusted_slot);
        let inputs = match preprocessor.run().await {
            Ok(inputs) => inputs,
            Err(e) => {
                println!("[Warning]: {:?}", e);
                tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                continue;
            }
        };

        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&inputs);

        // Generate Helios proof
        let helios_proof = match client
            .prove(&helios_pk, &stdin)
            .groth16()
            .run()
            .context("Failed to prove")
        {
            Ok(proof) => proof,
            Err(e) => {
                println!("Proof failed with error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(DEFAULT_TIMEOUT)).await;
                continue;
            }
        };

        let helios_outputs: HeliosOutputs =
            HeliosOutputs::abi_decode(&helios_proof.public_values.to_vec(), false).unwrap();

        let electra_block =
            get_electra_block(helios_outputs.newHead.try_into()?, &consensus_url).await;
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

        let previous_proof = service_state.most_recent_recursive_proof.clone();

        let recursion_inputs = RecursionCircuitInputs {
            electra_body_roots,
            electra_header,
            helios_proof: helios_proof.bytes(),
            helios_public_values: helios_proof.public_values.to_vec(),
            recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
            recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
            recursive_vk: recursive_vk.bytes32(),
            previous_head: service_state.trusted_slot,
        };

        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());

        // Run recursive proof in isolated task
        let recursive_proof = {
            let recursive_pk_clone = recursive_pk.clone();
            let stdin_clone = stdin.clone();
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

        let wrapper_inputs = WrapperCircuitInputs {
            recursive_proof: recursive_proof.bytes(),
            recursive_public_values: recursive_proof.public_values.to_vec(),
        };

        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&borsh::to_vec(&wrapper_inputs).unwrap());

        // Run wrapper proof in isolated task
        let final_wrapped_proof = {
            let wrapper_pk_clone = wrapper_pk.clone();
            let stdin_clone = stdin.clone();
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

        let wrapped_outputs: RecursionCircuitOutputs =
            borsh::from_slice(&recursive_proof.public_values.to_vec()).unwrap();

        service_state.most_recent_recursive_proof = Some(recursive_proof.clone());
        service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
        service_state.trusted_slot = helios_outputs.newHead.try_into().unwrap();
        service_state.trusted_height = wrapped_outputs.height;
        service_state.trusted_root = wrapped_outputs.root.try_into().unwrap();
        service_state.update_counter += 1;

        state_manager.save_state(&service_state)?;
        println!("New Service State: {:?} \n", service_state);
        println!("Alive for: {:?}", start_time.elapsed());
    }
}
