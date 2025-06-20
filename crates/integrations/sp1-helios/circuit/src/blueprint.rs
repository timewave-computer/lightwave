// This is the main recursion circuit that verifies Helios light client updates and maintains
// a chain of proofs for state transitions. It verifies both the Helios proof and previous
// wrapper proofs to ensure continuity of the light client state.

#![no_main]
sp1_zkvm::entrypoint!(main);
use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use beacon_electra::merkleize_header;
use helios_recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_verifier::Groth16Verifier;

// The trusted sync committee hash that was active at the trusted slot.
// This is used to verify the initial state when starting from the trusted slot.
const TRUSTED_SYNC_COMMITTEE_HASH: [u8; 32] = { committee_hash };

// The trusted slot number from which we start our light client chain.
// This must be a slot where we have verified the sync committee hash.
const TRUSTED_HEAD: u64 = { trusted_head };
const HELIOS_VK: &str = "{ helios_vk }";

pub fn main() {
    // Deserialize the circuit inputs which contain the Helios proof and previous wrapper proof
    let inputs: RecursionCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");

    // Get the Groth16 verification key for proof verification
    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;

    // Compute the Merkle root of the Electra block header
    let electra_block_header_root = merkleize_header(inputs.electra_header.clone());
    let electra_body_root = inputs.electra_body_roots.merkelize();
    let state_root = inputs.electra_body_roots.payload_roots.state_root;
    let height = inputs.electra_body_roots.payload_roots.block_number;

    // Decode the Helios proof outputs which contain the new header information
    let helios_output: HeliosOutputs =
        HeliosOutputs::abi_decode(&inputs.helios_public_values, false).unwrap();

    // Verify that the body root in the header matches our computed body root
    assert_eq!(inputs.electra_header.body_root, electra_body_root);

    // Verify that the header root matches the one from the Helios light client
    assert_eq!(
        electra_block_header_root.to_vec(),
        helios_output.newHeader.to_vec()
    );

    // Verify the Helios proof using Groth16 verification
    Groth16Verifier::verify(
        &inputs.helios_proof,
        &inputs.helios_public_values,
        // todo: hardcode this verifying key (must be the Helios VK)
        HELIOS_VK,
        groth16_vk,
    )
    .expect("Failed to verify helios zk light client update");

    if inputs.previous_head == TRUSTED_HEAD {
        // If this is the first proof after the trusted slot, verify the sync committee hash
        assert_eq!(
            helios_output.prevSyncCommitteeHash.to_vec(),
            TRUSTED_SYNC_COMMITTEE_HASH
        );

        let outputs = get_helios_outputs(helios_output, None, &inputs, &state_root, &height);

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    } else {
        // For subsequent proofs, verify the previous wrapper proof to ensure continuity
        Groth16Verifier::verify(
            &inputs
                .recursive_proof
                .as_ref()
                .expect("Previous proof is not provided"),
            &inputs
                .recursive_public_values
                .as_ref()
                .expect("Previous public values is not provided"),
            &inputs.recursive_vk,
            groth16_vk,
        )
        .expect("Failed to verify previous proof");

        // deserialize the inputs required for the recursive verification
        let recursive_proof_outputs: RecursionCircuitOutputs = borsh::from_slice(
            &inputs
                .recursive_public_values
                .as_ref()
                .expect("Previous public values is not provided"),
        )
        .unwrap();

        let outputs = get_helios_outputs(
            helios_output,
            Some(recursive_proof_outputs),
            &inputs,
            &state_root,
            &height,
        );

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    }
}

fn get_helios_outputs(
    helios_output: HeliosOutputs,
    recursive_proof_outputs: Option<RecursionCircuitOutputs>,
    recursive_proof_inputs: &RecursionCircuitInputs,
    state_root: &[u8; 32],
    height: &[u8; 32],
) -> RecursionCircuitOutputs {
    // Assert that the previous committee of the new proof matches the expected active committee
    if recursive_proof_outputs.is_some() {
        let recursive_proof_outputs =
            recursive_proof_outputs.expect("Failed to unwrap recursive proof outputs");

        // the new head must be greater than the previous head
        assert!(helios_output.prevHead < helios_output.newHead);

        // if the new head is for a new perid, the previous committee hash must match
        // the active committee hash of the previous proof
        if helios_output.prevHead / U256::from(8192) < helios_output.newHead / U256::from(8192) {
            if helios_output.prevSyncCommitteeHash != recursive_proof_outputs.active_committee {
                panic!("Sync committee mismatch!");
            }
        } else {
            // if the new head is for the same period, the previous committee hash must match
            // the previous committee hash of the previous proof
            // e.g. they should have the same previous committee hash for the same period
            if helios_output.prevSyncCommitteeHash != recursive_proof_outputs.previous_committee {
                panic!("Sync committee mismatch!");
            }
        }
    }

    // Commit the outputs required by the wrapper circuit
    RecursionCircuitOutputs {
        active_committee: helios_output
            .syncCommitteeHash
            .to_vec()
            .try_into()
            .expect("Failed to fit committeeHash into slice"),
        previous_committee: helios_output
            .prevSyncCommitteeHash
            .to_vec()
            .try_into()
            .expect("Failed to unwrap recursive proof outputs"),
        root: state_root.to_vec().try_into().unwrap(),
        height: unpad_block_number(height),
        vk: recursive_proof_inputs.recursive_vk.clone(),
    }
}

// the block height leaf in the merkle tree was padded to 32 bytes, so we need to unpad it
fn unpad_block_number(padded: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&padded[..8]);
    u64::from_le_bytes(bytes)
}
