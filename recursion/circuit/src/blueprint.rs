// This is the main recursion circuit that verifies Helios light client updates and maintains
// a chain of proofs for state transitions. It verifies both the Helios proof and previous
// wrapper proofs to ensure continuity of the light client state.

#![no_main]
sp1_zkvm::entrypoint!(main);
use alloy_sol_types::SolValue;
use beacon_electra::merkleize_header;
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_verifier::Groth16Verifier;

// The trusted sync committee hash that was active at the trusted slot.
// This is used to verify the initial state when starting from the trusted slot.
const TRUSTED_SYNC_COMMITTEE_HASH: [u8; 32] = { committee_hash };

// The trusted slot number from which we start our light client chain.
// This must be a slot where we have verified the sync committee hash.
const TRUSTED_HEAD: u64 = { trusted_head };
const HELIOS_VK: &str = "0x00e8ef401d89cf6c4698607644e75f1871724d56f7374972a6a5b76d3cdaf81e";
// Number of epochs before the next period to start using the next sync committee
const EPOCHS_BEFORE_NEXT_PERIOD: u64 = 10;

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

        let new_proof_active_committee: [u8; 32] = {
            // Calculate current epoch from update slot (32 slots per epoch)
            let new_slot: u64 = helios_output
                .newHead
                .try_into()
                .expect("Failed to fit new slot into u64");
            let current_epoch = new_slot / 32;
            // Calculate epochs until next period (assuming 256 epochs per period)
            let epochs_until_next_period = 256 - (current_epoch % 256);

            if epochs_until_next_period <= EPOCHS_BEFORE_NEXT_PERIOD
                && helios_output.nextSyncCommitteeHash != [0u8; 32]
            {
                helios_output
                    .nextSyncCommitteeHash
                    .to_vec()
                    .try_into()
                    .expect("Failed to fit nextSyncCommitteeHash into slice")
            } else {
                helios_output
                    .syncCommitteeHash
                    .to_vec()
                    .try_into()
                    .expect("Failed to fit committeeHash into slice")
            }
        };

        // Commit the outputs required by the wrapper circuit
        let outputs = RecursionCircuitOutputs {
            active_committee: new_proof_active_committee,
            root: state_root.to_vec().try_into().unwrap(),
            height: unpad_block_number(&height),
            vk: inputs.recursive_vk,
        };

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    } else {
        // For subsequent proofs, verify the previous wrapper proof to ensure continuity
        Groth16Verifier::verify(
            &inputs
                .recursive_proof
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
                .expect("Previous public values is not provided"),
        )
        .unwrap();

        let new_proof_active_committee: [u8; 32] = {
            // Calculate current epoch from update slot (32 slots per epoch)
            let new_slot: u64 = helios_output
                .newHead
                .try_into()
                .expect("Failed to fit new slot into u64");
            let current_epoch = new_slot / 32;
            // Calculate epochs until next period (assuming 256 epochs per period)
            let epochs_until_next_period = 256 - (current_epoch % 256);

            if epochs_until_next_period <= EPOCHS_BEFORE_NEXT_PERIOD
                && helios_output.nextSyncCommitteeHash != [0u8; 32]
            {
                helios_output
                    .nextSyncCommitteeHash
                    .to_vec()
                    .try_into()
                    .expect("Failed to fit nextSyncCommitteeHash into slice")
            } else {
                helios_output
                    .syncCommitteeHash
                    .to_vec()
                    .try_into()
                    .expect("Failed to fit committeeHash into slice")
            }
        };

        // Assert that the previous committee of the new proof matches the expected active committee
        if helios_output.prevSyncCommitteeHash != recursive_proof_outputs.active_committee {
            panic!(
                "[Warning] Sync committee mismatch, we might be at a boundary. Wait for 70 minutes and if this issue does not resolve itself, then there is a bug in the circuit!"
            );
        }

        // Commit the outputs required by the wrapper circuit
        let outputs = RecursionCircuitOutputs {
            active_committee: new_proof_active_committee,
            root: state_root.to_vec().try_into().unwrap(),
            height: unpad_block_number(&height),
            vk: inputs.recursive_vk,
        };

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    }
}

// Helper function to convert the padded block number from SSZ format to a u64
// SSZ uses little-endian for uint64 and pads to 32 bytes
fn unpad_block_number(padded: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&padded[..8]); // SSZ uses little-endian for uint64
    u64::from_le_bytes(bytes)
}
