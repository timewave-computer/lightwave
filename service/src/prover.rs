use alloy_sol_types::SolType;
use anyhow::{Context, Result};
use beacon_electra::{
    extract_electra_block_body, get_beacon_block_header, get_electra_block,
    types::electra::ElectraBlockHeader,
};
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs, WrapperCircuitInputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::time::Instant;

use crate::{
    HELIOS_ELF,
    preprocessor::Preprocessor,
    state::{ServiceState, StateManager},
};

/// Runs the main service loop that generates and verifies proofs
pub async fn run_prover_loop(
    state_manager: StateManager,
    mut service_state: ServiceState,
    recursive_elf: Vec<u8>,
    wrapper_elf: Vec<u8>,
    consensus_url: String,
) -> Result<()> {
    let client = ProverClient::new();
    let start_time = Instant::now();

    loop {
        // Set up the proving keys and verification keys for all circuits
        let (helios_pk, _) = client.setup(HELIOS_ELF);
        let (recursive_pk, recursive_vk) = client.setup(&recursive_elf);
        let (wrapper_pk, _) = client.setup(&wrapper_elf);

        println!("[Debug] Recursive VK: {:?}", recursive_vk.bytes32());
        // Initialize the preprocessor with the current trusted slot
        let preprocessor = Preprocessor::new(service_state.trusted_slot);

        // Get the next block's inputs for proof generation
        let inputs = match preprocessor.run().await {
            Ok(inputs) => inputs,
            Err(e) => {
                println!("[Warning]: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
        };

        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&inputs);

        // Generate the Helios proof
        let helios_proof = match client
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

        // Decode the Helios proof outputs
        let helios_outputs: HeliosOutputs =
            HeliosOutputs::abi_decode(&helios_proof.public_values.to_vec(), false).unwrap();

        // Fetch additional block data needed for execution payload (state_root, height) verification
        let electra_block =
            get_electra_block(helios_outputs.newHead.try_into()?, &consensus_url).await;

        // Extract and process block data
        let electra_body_roots = extract_electra_block_body(electra_block);
        let beacon_header =
            get_beacon_block_header(helios_outputs.newHead.try_into()?, &consensus_url).await;

        // Construct the zk-friendly Electra block header
        let electra_header = ElectraBlockHeader {
            slot: beacon_header.slot.as_u64(),
            proposer_index: beacon_header.proposer_index,
            parent_root: beacon_header.parent_root.to_vec().try_into().unwrap(),
            state_root: beacon_header.state_root.to_vec().try_into().unwrap(),
            body_root: beacon_header.body_root.to_vec().try_into().unwrap(),
        };

        // Get the previous proof if this isn't the first update
        let previous_proof = service_state.most_recent_recursive_proof;

        let recursion_inputs = RecursionCircuitInputs {
            electra_body_roots: electra_body_roots,
            electra_header: electra_header,
            helios_proof: helios_proof.bytes(),
            helios_public_values: helios_proof.public_values.to_vec(),
            recursive_proof: previous_proof.as_ref().map(|p| p.bytes()),
            recursive_public_values: previous_proof.as_ref().map(|p| p.public_values.to_vec()),
            recursive_vk: recursive_vk.bytes32(),
            previous_head: service_state.trusted_slot,
        };

        // Generate the recursive proof
        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&borsh::to_vec(&recursion_inputs).unwrap());

        let recursive_proof = client
            .prove(&recursive_pk, &stdin)
            .groth16()
            .run()
            .context("Failed to prove")?;

        let wrapper_inputs = WrapperCircuitInputs {
            recursive_proof: recursive_proof.bytes(),
            recursive_public_values: recursive_proof.public_values.to_vec(),
        };

        // Generate the recursive proof
        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&borsh::to_vec(&wrapper_inputs).unwrap());

        // the final wrapped proof to send to the coprocessor
        let final_wrapped_proof = client
            .prove(&wrapper_pk, &stdin)
            .groth16()
            .run()
            .context("Failed to prove")?;

        // Decode the recursive proof outputs
        let wrapped_outputs: RecursionCircuitOutputs =
            borsh::from_slice(&recursive_proof.public_values.to_vec()).unwrap();

        // Update the service state with new trusted information
        service_state.most_recent_recursive_proof = Some(recursive_proof.clone());

        // this is the proof that should be returned by the API endpoint get_proof
        service_state.most_recent_wrapper_proof = Some(final_wrapped_proof);
        service_state.trusted_slot = helios_outputs.newHead.try_into().unwrap();
        service_state.trusted_height = wrapped_outputs.height;
        service_state.trusted_root = wrapped_outputs.root.try_into().unwrap();
        service_state.update_counter += 1;

        // Save the updated state to the database
        state_manager.save_state(&service_state)?;

        // Log the updated state and elapsed time
        println!("New Service State: {:?} \n", service_state);
        let elapsed_time = start_time.elapsed();
        println!("Alive for: {:?}", elapsed_time);
    }
}
