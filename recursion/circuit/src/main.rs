#![no_main]
sp1_zkvm::entrypoint!(main);
use alloy_sol_types::SolValue;
use recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
use sp1_helios_primitives::types::ProofOutputs as HeliosOutputs;
use sp1_verifier::Groth16Verifier;

const TRUSTED_SYNC_COMMITTEE_HASH: [u8; 32] = [0; 32];
const TRUSTED_HEAD: u64 = 7561216 - (32 * 254);

pub fn main() {
    let inputs: RecursionCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");
    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;

    if inputs.previous_head == TRUSTED_HEAD {
        let helios_output: HeliosOutputs =
            HeliosOutputs::abi_decode(&inputs.helios_public_values, false).unwrap();
        assert_eq!(
            helios_output.prevSyncCommitteeHash.to_vec(),
            TRUSTED_SYNC_COMMITTEE_HASH
        );
        // verify the helios proof
        Groth16Verifier::verify(
            &inputs.helios_proof,
            &inputs.helios_public_values,
            &inputs.helios_vk,
            groth16_vk,
        )
        .expect("Failed to verify helios zk light client update");
        let outputs = RecursionCircuitOutputs {
            root: helios_output.newHeader.to_vec(),
            slot: helios_output.newHead.try_into().unwrap(),
        };
        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    } else {
        let helios_output: HeliosOutputs =
            HeliosOutputs::abi_decode(&inputs.helios_public_values, false).unwrap();
        // verify the previous proof
        Groth16Verifier::verify(
            &inputs
                .previous_proof
                .expect("Previous proof is not provided"),
            &inputs
                .previous_public_values
                .expect("Previous proof is not provided"),
            // todo: hardcode this verifying key (must be the Recursive circuit VK)
            &inputs.previous_vk.expect("Previous proof is not provided"),
            groth16_vk,
        )
        .expect("Failed to verify previous proof");
        // verify the helios proof
        Groth16Verifier::verify(
            &inputs.helios_proof,
            &inputs.helios_public_values,
            // todo: hardcode this verifying key (must be the Helios VK)
            &inputs.helios_vk,
            groth16_vk,
        )
        .expect("Failed to verify helios zk light client update");
        let outputs = RecursionCircuitOutputs {
            root: helios_output.newHeader.to_vec(),
            slot: helios_output.newHead.try_into().unwrap(),
        };
        sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
    }
}
