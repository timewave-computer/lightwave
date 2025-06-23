// This is the main recursion circuit that verifies Tendermint light client updates and maintains
// a chain of proofs for state transitions. It verifies both the Tendermint proof and previous
// wrapper proofs to ensure continuity of the light client state.

#![no_main]

use sp1_tendermint_primitives::TendermintOutput;
use sp1_verifier::Groth16Verifier;
use tendermint_recursion_types::{RecursionCircuitInputs, RecursionCircuitOutputs};
sp1_zkvm::entrypoint!(main);

// The trusted slot number from which we start our light client chain.
// This must be a slot where we have verified the sync committee hash.
const TRUSTED_HEIGHT: u64 = 31134400;
const TRUSTED_ROOT: [u8; 32] = [133, 197, 217, 208, 182, 161, 40, 102, 214, 74, 216, 44, 87, 164, 134, 95, 150, 222, 115, 170, 222, 9, 183, 138, 57, 107, 86, 21, 40, 96, 131, 113];
const TENDERMINT_VK: &str = "0x00be33671b715fb3f8657ae631b2a7032e2ecda1fc598d18ac234f87ba2a8fd5";

pub fn main() {
    // Deserialize the circuit inputs which contain the Tendermint proof and previous wrapper proof
    let inputs: RecursionCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");

    let tendermintx_output: TendermintOutput =
        serde_json::from_slice(&inputs.tendermint_public_values)
            .expect("Failed to deserialize Tendermint Output");

    // Get the Groth16 verification key for proof verification
    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;

    // Verify the Tendermint proof
    Groth16Verifier::verify(
        &inputs.tendermint_proof,
        &inputs.tendermint_public_values,
        TENDERMINT_VK,
        groth16_vk,
    )
    .expect("Failed to verify Tendermint proof");
    if inputs.trusted_height == TRUSTED_HEIGHT {
        assert_eq!(tendermintx_output.trusted_header_hash, TRUSTED_ROOT);
    } else {
        let recusive_proof_outputs: RecursionCircuitOutputs = borsh::from_slice(
            &inputs
                .recursive_public_values
                .as_ref()
                .expect("Failed to unwrap recursive public values"),
        )
        .expect("Failed to deserialize Recursive Outputs");
        assert!(tendermintx_output.target_height > recusive_proof_outputs.height);
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
    }
    let outputs = RecursionCircuitOutputs {
        root: tendermintx_output.target_header_hash,
        height: tendermintx_output.target_height,
        vk: inputs.recursive_vk,
    };
    sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
}
