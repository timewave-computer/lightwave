#![no_main]
sp1_zkvm::entrypoint!(main);
use recursion_types::WrapperCircuitInputs;
use sp1_verifier::Groth16Verifier;
fn main() {
    let groth16_vk: &[u8] = *sp1_verifier::GROTH16_VK_BYTES;
    let inputs: WrapperCircuitInputs =
        borsh::from_slice(&sp1_zkvm::io::read_vec()).expect("Failed to deserialize Inputs");
    let public_outputs = inputs.recursive_public_values;
    Groth16Verifier::verify(
        &inputs.recursive_proof,
        &public_outputs,
        // todo: hardcode this verifying key (must be the Recursive circuit VK)
        &inputs.recursive_vk,
        groth16_vk,
    )
    .expect("Failed to verify previous proof");

    // re-commit the public outputs after recursive proof verification
    sp1_zkvm::io::commit_slice(&public_outputs);
}
