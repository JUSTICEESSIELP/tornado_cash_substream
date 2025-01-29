use ethers::prelude::*;
use ethers::utils::keccak256;
use ethers::types::H256;

pub fn get_event_signature() -> H256 {
    // Define the event signature (match the event definition)
    let event_signature = "Deposit(bytes,uint32,uint64)";
    
    // Compute keccak256 hash
    let signature_hash = keccak256(event_signature.as_bytes());

    // Return the signature hash
    H256::from(signature_hash)
}