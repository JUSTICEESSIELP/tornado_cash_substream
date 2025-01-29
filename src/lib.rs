mod utils; // Include the module for event signature calculation
mod pb;

use substreams::store::{StoreAdd, StoreNew, StoreGet};
use substreams_ethereum::pb::eth::v2::Block;
use substreams::errors::Error;
use std::collections::HashMap;
use hex_literal::hex;
use ethers::types::H256;
use utils::event_signature::get_event_signature

// Contract address for the ZK pool
const TORNADO_POOL_ADDRESS: &str = "0x722122dF12D4e14e13Ac3b6895a86e84145b6967";

#[substreams::handlers::store]
fn store_deposits(block: &Block, store: StoreAddInt64) {
    // Event signature for Deposit event
    let deposit_event_signature: H256 = get_event_signature("Deposit", "bytes,uint32,uint64");

    // Process deposit events
    for trx in block.transactions() {
        for log in trx.logs {
            if log.address == TORNADO_POOL_ADDRESS {
                // Match Deposit event signature
                if log.topics[0] == deposit_event_signature {
                    let commitment = log.topics[1];
                    let leaf_index = u32::from_be_bytes(log.data[0..4].try_into().unwrap());
                    
                    // Store total deposits
                    store.add("total_deposits", 1);
                    
                    // Store unique commitments
                    store.set(format!("commitment:{}", hex::encode(commitment)), 1);
                    
                    // Store deposits by hour
                    let hour = block.timestamp.as_ref().unwrap().seconds / 3600;
                    store.add(format!("deposits_hour:{}", hour), 1);
                }
            }
        }
    }
}

#[substreams::handlers::store]
fn store_withdrawals(block: &Block, store: StoreAddInt64) {
    // Event signature for Withdrawal event
    let withdrawal_event_signature: H256 = get_event_signature("Withdrawal", "bytes,uint32,uint64");

    for trx in block.transactions() {
        for log in trx.logs {
            if log.address == TORNADO_POOL_ADDRESS {
                // Match Withdrawal event signature
                if log.topics[0] == withdrawal_event_signature {
                    let nullifier_hash = log.topics[1];
                    let relayer = &log.topics[2];
                    let fee = u256::from_big_endian(&log.data[0..32]);
                    
                    // Store total withdrawals
                    store.add("total_withdrawals", 1);
                    
                    // Store unique nullifiers
                    store.set(format!("nullifier:{}", hex::encode(nullifier_hash)), 1);
                    
                    // Track relayer activity
                    store.add(format!("relayer:{}", hex::encode(relayer)), 1);
                    
                    // Accumulate fees
                    store.add("total_fees", fee.as_u64());
                }
            }
        }
    }
}

#[substreams::handlers::map]
fn map_pool_stats(deposits: Stores, withdrawals: Stores) -> Result<PoolStats, Error> {
    let mut stats = PoolStats::default();
    
    // Aggregate deposit metrics
    stats.total_deposits = deposits.get_last("total_deposits")?.unwrap_or(0);
    
    // Aggregate withdrawal metrics
    stats.total_withdrawals = withdrawals.get_last("total_withdrawals")?.unwrap_or(0);
    stats.total_fees = withdrawals.get_last("total_fees")?.unwrap_or(0).to_string();
    
    // Calculate active relayers
    let mut relayers = HashMap::new();
    for key in withdrawals.keys_by_prefix("relayer:") {
        let count = withdrawals.get_last(&key)?.unwrap_or(0);
        relayers.insert(key[8..].to_string(), count);
    }
    stats.active_relayers = relayers;
    
    Ok(stats)
}
