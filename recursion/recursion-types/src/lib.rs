use beacon_electra::types::electra::{ElectraBlockBodyRoots, ElectraBlockHeader};
use borsh::{BorshDeserialize, BorshSerialize};
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitInputs {
    pub electra_body_roots: ElectraBlockBodyRoots,
    pub electra_header: ElectraBlockHeader,
    pub helios_proof: Vec<u8>,
    pub helios_public_values: Vec<u8>,
    pub helios_vk: String,
    pub previous_wrapper_proof: Option<Vec<u8>>,
    pub previous_wrapper_public_values: Option<Vec<u8>>,
    pub previous_wrapper_vk: Option<String>,
    pub previous_head: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct WrapperCircuitInputs {
    pub recursive_proof: Vec<u8>,
    pub recursive_public_values: Vec<u8>,
    pub recursive_vk: String,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitOutputs {
    // the execution state root
    pub root: Vec<u8>,
    // the height of the execution block
    pub height: u64,
}
