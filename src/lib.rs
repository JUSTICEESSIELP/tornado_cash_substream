mod utils; // Include the module for event signature calculation
mod pb;
mod abi


use substreams::pb::substreams::store_delta::Operation;
use substreams::store::{StoreGet, StoreGetInt64, StoreAdd, StoreNew};
use substreams_ethereum::pb::eth::v2::Block;
use pb::tornado::{Withdrawal, Deposit, DepositMetrics}
use substreams::errors::Error;
use std::collections::HashMap;
use hex_literal::hex;
use ethers::types::H256;
use substreams::errors::Error;
use substreams::log;
use hex::encode;

// Contract address for the ZK pool

const TORNADO_POOL_ADDRESS:[u8;20]= hex!("722122dF12D4e14e13Ac3b6895a86e84145b6967");
// Handler for additive metrics (counts, totals, fees)
#[substreams::handlers::store]
fn store_additive_metrics(block: &Block, store: StoreAddInt64) {
    // Handle deposits
    block
        .events::<abi::tornado_cash::events::Deposit>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(_, log)| {
            let log_ordinal = log.block_index() as u64;
            let block_time = block.timestamp.as_ref().unwrap().seconds;
            
            // Track total deposits
            store.add(log_ordinal, "total_deposits", 1);
            
            // Track deposits by hour
            let hour = block_time / 3600;
            store.add(log_ordinal, format!("deposits_hour:{}", hour), 1);
            
            // Track deposits by day
            let day = block_time / 86400;
            store.add(log_ordinal, format!("deposits_day:{}", day), 1);
        });

    // Handle withdrawals
    block
        .events::<abi::tornado_cash::events::Withdrawal>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(withdrawal, log)| {
            let log_ordinal = log.block_index() as u64;
            let block_time = block.timestamp.as_ref().unwrap().seconds;
            
            // Track total withdrawals
            store.add(log_ordinal, "total_withdrawals", 1);
            
            // Track withdrawals by hour/day
            let hour = block_time / 3600;
            let day = block_time / 86400;
            store.add(log_ordinal, format!("withdrawals_hour:{}", hour), 1);
            store.add(log_ordinal, format!("withdrawals_day:{}", day), 1);
            
            // Track relayer activity
            let relayer_hex = encode(&withdrawal.relayer);
            store.add(
                log_ordinal,
                format!("relayer_withdrawals:{}", relayer_hex),
                1
            );
            
            // Track fees
            store.add(log_ordinal, "total_fees", withdrawal.fee.to_u64());
            store.add(
                log_ordinal,
                format!("relayer_fees:{}", relayer_hex),
                withdrawal.fee.to_u64()
            );
            
            // Track recipient activity
            let recipient_hex = encode(&withdrawal.to);
            store.add(
                log_ordinal,
                format!("recipient_withdrawals:{}", recipient_hex),
                1
            );
        });
}

// Handler for set-based metrics (unique identifiers)
#[substreams::handlers::store]
fn store_unique_identifiers(block: &Block, store: StoreSetString) {
    // Track unique deposit commitments
    block
        .events::<abi::tornado_cash::events::Deposit>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(deposit, log)| {
            let log_ordinal = log.block_index() as u64;
            let commitment_hex = encode(deposit.commitment);
            store.set(log_ordinal, format!("commitment:{}", commitment_hex), "1");
        });

    // Track unique withdrawal nullifiers
    block
        .events::<abi::tornado_cash::events::Withdrawal>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(withdrawal, log)| {
            let log_ordinal = log.block_index() as u64;
            let nullifier_hex = encode(withdrawal.nullifier_hash);
            store.set(log_ordinal, format!("nullifier:{}", nullifier_hex), "1");
        });
}

// Optional: Handler for max values if needed
#[substreams::handlers::store]
fn store_max_values(block: &Block, store: StoreMaxInt64) {
    block
        .events::<abi::tornado_cash::events::Withdrawal>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(withdrawal, log)| {
            let log_ordinal = log.block_index() as u64;
            let relayer_hex = encode(&withdrawal.relayer);
            
            // Track maximum fee per relayer
            store.set(
                log_ordinal,
                format!("max_relayer_fee:{}", relayer_hex),
                withdrawal.fee.to_u64()
            );
        });
}

#[substreams::handlers::map]
fn map_tornado_metrics(
    block: &Block,
    additive_store: StoreGetInt64,
    max_store: StoreGetInt64,
) -> Result<pb::tornado::DepositMetrics, Error> {
    let block_number = block.number;
    let timestamp = block.timestamp.as_ref().unwrap().seconds;
    
    // Initialize metrics object
    let mut metrics = pb::tornado::DepositMetrics {
        total_deposits: 0,
        total_withdrawals: 0,
        total_fees: 0,
        relayer_metrics: vec![],
    };
    
    // Get current total deposits
    if let Some(deposits) = additive_store.get_at("total_deposits", block_number) {
        metrics.total_deposits = deposits as u64;
    }
    
    // Get current total withdrawals
    if let Some(withdrawals) = additive_store.get_at("total_withdrawals", block_number) {
        metrics.total_withdrawals = withdrawals as u64;
    }
    
    // Get total fees
    if let Some(fees) = additive_store.get_at("total_fees", block_number) {
        metrics.total_fees = fees as u64;
    }
    
    // Process relayer metrics
    let mut relayer_map = std::collections::HashMap::new();
    
    // Collect relayer withdrawals and fees
    block
        .events::<abi::tornado_cash::events::Withdrawal>(&[&TORNADO_POOL_ADDRESS])
        .for_each(|(withdrawal, _)| {
            let relayer_hex = encode(&withdrawal.relayer);
            
            let relayer_metrics = relayer_map
                .entry(relayer_hex.clone())
                .or_insert_with(|| pb::tornado::RelayerMetrics {
                    address: relayer_hex.clone(),
                    total_withdrawals: 0,
                    total_fees: 0,
                    max_fee: 0,
                });
            
            // Get relayer's total withdrawals
            if let Some(withdrawals) = additive_store.get_at(
                format!("relayer_withdrawals:{}", relayer_hex),
                block_number,
            ) {
                relayer_metrics.total_withdrawals = withdrawals as u64;
            }
            
            // Get relayer's total fees
            if let Some(fees) = additive_store.get_at(
                format!("relayer_fees:{}", relayer_hex),
                block_number,
            ) {
                relayer_metrics.total_fees = fees as u64;
            }
            
            // Get relayer's max fee
            if let Some(max_fee) = max_store.get_at(
                format!("max_relayer_fee:{}", relayer_hex),
                block_number,
            ) {
                relayer_metrics.max_fee = max_fee as u64;
            }
        });
    
    metrics.relayer_metrics = relayer_map.into_values().collect();
    
    Ok(metrics)
}

