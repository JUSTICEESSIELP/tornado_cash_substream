// src/lib.rs
use substreams::prelude::*;
use substreams::{log, store, Hex};
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

// Generated proto types
mod pb;
use pb::tornado::v1::{Deposit, Withdrawal, DepositMetrics, WithdrawalMetrics, PoolStats};

const TORNADO_POOL_ADDRESS: &str = "0x722122dF12D4e14e13Ac3b6895a86e84145b6967";

#[substreams::handlers::store]
fn store_deposits(block: eth::Block) -> Result<store::StoreSetProto<DepositMetrics>, substreams::errors::Error> {
    let mut metrics = DepositMetrics {
        total_deposits: 0,
        unique_commitments: 0,
        deposits_by_hour: Default::default(),
    };

    for log in block.logs() {
        if log.address != TORNADO_POOL_ADDRESS {
            continue;
        }

        if let Some(deposit) = Deposit::decode_event(log) {
            metrics.total_deposits += 1;
            
            let hour = (block.timestamp().seconds / 3600).to_string();
            *metrics.deposits_by_hour.entry(hour).or_insert(0) += 1;
        }
    }

    Ok(store::StoreSetProto(metrics))
}

#[substreams::handlers::store]
fn store_withdrawals(block: eth::Block) -> Result<store::StoreSetProto<WithdrawalMetrics>, substreams::errors::Error> {
    let mut metrics = WithdrawalMetrics {
        total_withdrawals: 0,
        unique_nullifiers: 0,
        total_fees: "0".to_string(),
        withdrawals_by_hour: Default::default(),
    };

    for log in block.logs() {
        if log.address != TORNADO_POOL_ADDRESS {
            continue;
        }

        if let Some(withdrawal) = Withdrawal::decode_event(log) {
            metrics.total_withdrawals += 1;
            metrics.unique_nullifiers += 1;
            
            // Add fee to total
            let fee = withdrawal.fee.parse::<u128>().unwrap_or(0);
            let current_total = metrics.total_fees.parse::<u128>().unwrap_or(0);
            metrics.total_fees = (current_total + fee).to_string();

            let hour = (block.timestamp().seconds / 3600).to_string();
            *metrics.withdrawals_by_hour.entry(hour).or_insert(0) += 1;
        }
    }

    Ok(store::StoreSetProto(metrics))
}

#[substreams::handlers::map]
fn map_pool_stats(
    deposits: store::StoreGetProto<DepositMetrics>,
    withdrawals: store::StoreGetProto<WithdrawalMetrics>,
) -> Result<PoolStats, substreams::errors::Error> {
    let deposits = deposits.get();
    let withdrawals = withdrawals.get();

    Ok(PoolStats {
        total_deposits: deposits.total_deposits,
        total_withdrawals: withdrawals.total_withdrawals,
        total_fees: withdrawals.total_fees.clone(),
        average_time_between_deposit_withdrawal: calculate_average_time(&deposits, &withdrawals),
        active_relayers: withdrawals.withdrawals_by_hour.clone(),
    })
}

fn calculate_average_time(deposits: &DepositMetrics, withdrawals: &WithdrawalMetrics) -> f64 {
    if deposits.total_deposits == 0 || withdrawals.total_withdrawals == 0 {
        return 0.0;
    }

    // Simple approximation based on first and last events
    // In a real implementation, you'd want to track individual deposit-withdrawal pairs
    let deposit_times: Vec<u64> = deposits.deposits_by_hour
        .keys()
        .filter_map(|k| k.parse::<u64>().ok())
        .collect();
    
    let withdrawal_times: Vec<u64> = withdrawals.withdrawals_by_hour
        .keys()
        .filter_map(|k| k.parse::<u64>().ok())
        .collect();

    if deposit_times.is_empty() || withdrawal_times.is_empty() {
        return 0.0;
    }

    let avg_deposit_time = deposit_times.iter().sum::<u64>() as f64 / deposit_times.len() as f64;
    let avg_withdrawal_time = withdrawal_times.iter().sum::<u64>() as f64 / withdrawal_times.len() as f64;

    (avg_withdrawal_time - avg_deposit_time) * 3600.0 // Convert hours to seconds
}