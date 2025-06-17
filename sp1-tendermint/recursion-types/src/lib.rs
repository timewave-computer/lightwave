#![no_std]
extern crate alloc;
use alloc::{string::String, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize};
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitInputs {
    pub tendermint_proof: Vec<u8>,
    pub tendermint_public_values: Vec<u8>,
    pub recursive_proof: Option<Vec<u8>>,
    pub recursive_public_values: Option<Vec<u8>>,
    pub recursive_vk: String,
    pub trusted_height: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct RecursionCircuitOutputs {
    pub root: [u8; 32],
    pub height: u64,
    pub vk: String,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct WrapperCircuitInputs {
    pub recursive_proof: Vec<u8>,
    pub recursive_public_values: Vec<u8>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct WrapperCircuitOutputs {
    pub height: u64,
    pub root: [u8; 32],
}
