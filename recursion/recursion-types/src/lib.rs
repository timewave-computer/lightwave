use borsh::{BorshDeserialize, BorshSerialize};
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitInputs {
    pub helios_proof: Vec<u8>,
    pub helios_public_values: Vec<u8>,
    pub helios_vk: String,
    pub previous_proof: Option<Vec<u8>>,
    pub previous_public_values: Option<Vec<u8>>,
    pub previous_vk: Option<String>,
    pub previous_head: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitOutputs {
    pub root: Vec<u8>,
    pub slot: u64,
}
