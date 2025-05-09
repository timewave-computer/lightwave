#![no_main]
sp1_zkvm::entrypoint!(main);
use alloy_sol_types::SolValue;
use beacon_electra::merkleize_header;
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_verifier::Groth16Verifier;

// must be initialized correctly
const TRUSTED_SYNC_COMMITTEE_HASH: [u8; 32] = [
    92, 237, 21, 16, 174, 107, 192, 53, 41, 51, 34, 165, 149, 128, 0, 195, 233, 139, 32, 74, 161,
    128, 52, 111, 65, 21, 178, 227, 105, 1, 225, 233,
];
// must be initialized correctly to the trusted slot
const TRUSTED_HEAD: u64 = 7584512;

pub fn main() {
    let inputs: RecursionCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");

    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;
    let electra_block_header_root = merkleize_header(inputs.electra_header.clone());
    let electra_body_root = inputs.electra_body_roots.merkelize();
    let state_root = inputs.electra_body_roots.payload_roots.state_root;
    let height = inputs.electra_body_roots.payload_roots.block_number;

    let helios_output: HeliosOutputs =
        HeliosOutputs::abi_decode(&inputs.helios_public_values, false).unwrap();

    // verify the block body root against that in the header
    assert_eq!(inputs.electra_header.body_root, electra_body_root);

    // verify the header root against the one from the ethereum zk light client
    assert_eq!(
        electra_block_header_root.to_vec(),
        helios_output.newHeader.to_vec()
    );

    // verify the helios proof
    Groth16Verifier::verify(
        &inputs.helios_proof,
        &inputs.helios_public_values,
        // todo: hardcode this verifying key (must be the Helios VK)
        &inputs.helios_vk,
        groth16_vk,
    )
    .expect("Failed to verify helios zk light client update");

    if inputs.previous_head == TRUSTED_HEAD {
        assert_eq!(
            helios_output.prevSyncCommitteeHash.to_vec(),
            TRUSTED_SYNC_COMMITTEE_HASH
        );

        let outputs = RecursionCircuitOutputs {
            root: state_root.to_vec(),
            height: unpad_block_number(&height),
        };

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    } else {
        // verify the previous wrapper proof

        // move this code into a wrapper cicuit because we can't derive the elf inside the same circuit
        Groth16Verifier::verify(
            &inputs
                .previous_wrapper_proof
                .expect("Previous proof is not provided"),
            &inputs
                .previous_wrapper_public_values
                .expect("Previous proof is not provided"),
            // todo: hardcode this verifying key (must be the Wrapper circuit VK)
            &inputs
                .previous_wrapper_vk
                .expect("Previous proof is not provided"),
            groth16_vk,
        )
        .expect("Failed to verify previous proof");

        let outputs = RecursionCircuitOutputs {
            root: state_root.to_vec(),
            height: unpad_block_number(&height),
        };

        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    }
}

// helper tounpad the block number leaf from the execution payload
fn unpad_block_number(padded: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&padded[..8]); // SSZ uses little-endian for uint64
    u64::from_le_bytes(bytes)
}
