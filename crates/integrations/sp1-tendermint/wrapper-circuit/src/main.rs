// This is the wrapper circuit that verifies recursive proofs from the main recursion circuit.
// It serves as a bridge between recursive proofs, ensuring that each new proof is properly
// verified against the previous one in the chain.

#![no_main]
sp1_zkvm::entrypoint!(main);
use sp1_verifier::Groth16Verifier;
use tendermint_recursion_types::{
    RecursionCircuitOutputs, WrapperCircuitInputs, WrapperCircuitOutputs,
};

const RECURSIVE_VK: &str = "0x009094b993417fd795f3785e430cc9153705f79c798ac8f337acfabad95d4edc";

fn main() {
    // Get the Groth16 verification key for proof verification
    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;

    // Deserialize the wrapper circuit inputs which contain the recursive proof
    let inputs: WrapperCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");

    let recursive_outputs: RecursionCircuitOutputs =
        borsh::from_slice(&inputs.recursive_public_values)
            .expect("Failed to deserialize recursive Outputs");

    // Assert that the VK used for the verification of the recursive proof (if any) matches
    // exactly the VK of the recursive circuit.
    // This is required for every proof except the first one.
    assert_eq!(recursive_outputs.vk, RECURSIVE_VK);
    // Get the public outputs from the recursive proof
    let public_outputs = inputs.recursive_public_values;

    // Verify the recursive proof using Groth16 verification
    Groth16Verifier::verify(
        &inputs.recursive_proof,
        &public_outputs,
        // todo: hardcode this verifying key (must be the Recursive circuit VK)
        RECURSIVE_VK,
        groth16_vk,
    )
    .expect("Failed to verify previous proof");

    // Re-commit the public outputs after recursive proof verification
    // This ensures the outputs are available for the next proof in the chain
    let outputs = WrapperCircuitOutputs {
        height: recursive_outputs.height,
        root: recursive_outputs.root,
    };
    sp1_zkvm::io::commit_slice(&borsh::to_vec(&outputs).unwrap());
}
