mod pb;
mod abi;
use substreams::store::StoreNew;
use substreams::store::{StoreAdd, StoreAddInt64, StoreGet, StoreGetInt64, StoreMaxInt64, StoreSetString};
use num_bigint::BigUint; 
// Import BigUint


use substreams_entity_change::{pb::entity::EntityChanges, tables::Tables};
// use substreams_ethereum::pb::sf::ethereum::r#type::v2 as eth;
use substreams_ethereum::pb::eth::v2 as eth;
use hex_literal::hex;
use crate::pb::tornado::types::v1::{ Withdrawal, Deposit, TornadoEvents, PoolMetrics};
use substreams::errors::Error;
use substreams::Hex; // Import Hex for encoding


// Contract address for the ZK pool
const TORNADO_POOL_ADDRESS: [u8; 20] = hex!("722122dF12D4e14e13Ac3b6895a86e84145b6967");

substreams_ethereum::init!();

#[substreams::handlers::map]
fn tornado_event_mapper(block: eth::Block) -> Result<Option<TornadoEvents>, Error> {
    let mut tornado_events = TornadoEvents {
        deposits: vec![],
        withdrawals: vec![],
    };

    let pool_address = TORNADO_POOL_ADDRESS.to_vec();

    for receipt_view in block.receipts() {
        let receipt = &receipt_view.receipt;
        let transaction = &receipt_view.transaction; // Get the transaction

        for log in &receipt.logs {
            if log.address.to_vec() == pool_address {
                if let Some(deposit_event) = abi::tornado_cash::events::Deposit::decode(log).ok() {
                    // let amount = transaction.value.as_ref().map_or("0".to_string(), |v| {
                    //     let big_uint = BigUint::from_bytes_be(&v.bytes);
                    //     big_uint.to_string()
                    // });

                  


                    let amount = transaction.value.as_ref().map_or("0".to_string(), |v| {
                        let big_uint = BigUint::from_bytes_be(&v.bytes);
                        let amount_str = big_uint.to_string();

                        match amount_str.parse::<u64>() {
                            Ok(val) => {
                                let result = val / 1_000_000_000_000_000_000; // Integer division
                                result.to_string()
                            }
                            Err(e) => {
                                substreams::log::info!("Failed to parse amount: {} - Error: {}", amount_str, e);
                                "0".to_string()
                            }
                        }
                    });
                 




                    let from = Hex::encode(&transaction.from); // & needed here
                    let commitment = Hex::encode(deposit_event.commitment);
                    let hash = Hex::encode(&transaction.hash); // Unwrap hash safely

                    tornado_events.deposits.push(Deposit {
                        commitment,
                        block_number: block.number,
                        block_time: Some(block.timestamp().to_owned()),
                        log_index: log.block_index,
                        amount,
                        from,
                        hash
                    });
                }

                if let Some(withdrawal_event) = abi::tornado_cash::events::Withdrawal::decode(log).ok() {
                    
                                       let fee_str = withdrawal_event.fee.to_string();
                    let fee = match fee_str.parse::<u64>() {
                        Ok(f) => f,
                        Err(e) => {
                            substreams::log::info!("Failed to parse fee: {} - Error: {}", fee_str, e);
                            0 // Or handle the error as you see fit (return an error, skip the event, etc.)
                        }
                    };
           
                    let nullifier_hash = Hex::encode(withdrawal_event.nullifier_hash);
                    let to = Hex::encode(withdrawal_event.to);
                    let relayer = Hex::encode(withdrawal_event.relayer);

                    tornado_events.withdrawals.push(Withdrawal {
                        nullifier_hash,
                        to,
                        relayer,
                        fee:fee.to_string(),
                        block_number: block.number,
                        block_time: Some(block.timestamp().to_owned()),
                        log_index: log.block_index,
                    });
                }
            }
        }
    }

    if tornado_events.deposits.is_empty() && tornado_events.withdrawals.is_empty() {
        Ok(None)
    } else {
        Ok(Some(tornado_events))
    }
}


#[substreams::handlers::store]
pub fn store_additive_metrics(events: TornadoEvents, output: StoreAddInt64) {
    for deposit_event in events.deposits {
        if let Some(block_time) = deposit_event.block_time {
            let log_ordinal = deposit_event.log_index as u64;
            
            output.add(log_ordinal, "total_deposits", 1);

            let hour = block_time.seconds / 3600;
            output.add(log_ordinal, &format!("deposits_hour:{}", hour), 1);

            let day = block_time.seconds / 86400;
            output.add(log_ordinal, &format!("deposits_day:{}", day), 1);
        }
    }

    for withdraw_event in events.withdrawals {
        if let Some(block_time) = withdraw_event.block_time {
            let log_ordinal = withdraw_event.log_index as u64;

            output.add(log_ordinal, "total_withdrawals", 1);

            let hour = block_time.seconds / 3600;
            let day = block_time.seconds / 86400;
            
            output.add(log_ordinal, &format!("withdrawals_hour:{}", hour), 1);
            output.add(log_ordinal, &format!("withdrawals_day:{}", day), 1);

            output.add(log_ordinal, &format!("relayer_withdrawals:{}", withdraw_event.relayer), 1);

            if let Ok(fee) = withdraw_event.fee.parse::<i64>() {
                output.add(log_ordinal, "total_fees_to_withdraw", fee);
            }

            output.add(
                log_ordinal,
                &format!("recipient_withdrawals:{}", withdraw_event.to),
                1
            );
        }
    }
}


#[substreams::handlers::map]
pub fn map_store_metrics(
    store: StoreGetInt64,
) -> Result<PoolMetrics, Error> {
    let total_deposits = store.get_last("total_deposits").unwrap_or_default();
    let total_withdrawals = store.get_last("total_withdrawals").unwrap_or_default();
    let total_fees = store.get_last("total_fees_to_withdraw").unwrap_or_default();


     substreams::log::info!("{:?}", total_deposits);
    Ok(PoolMetrics {
        total_deposits,
        total_withdrawals,
        total_fees,
    })
}

#[substreams::handlers::map]
pub fn graph_out(poolStats: PoolMetrics, events:TornadoEvents) -> Result<EntityChanges, substreams::errors::Error> {
    // hash map of name to a table
    let mut tables = Tables::new();

   // Create Pool Stats Entity
   tables
   .create_row("PoolStats", "pool_stats")
   .set("totalDeposits", poolStats.total_deposits)
   .set("totalWithdrawals", poolStats.total_withdrawals)
   .set("totalFees", poolStats.total_fees);

   for deposit in events.deposits {
    tables
        .create_row("Deposit", &deposit.hash) // Using hash as unique ID
        .set("commitment", deposit.commitment)
        .set("blockNumber", deposit.block_number)
        .set("timestamp", deposit.block_time.unwrap().seconds)
        .set("amount", deposit.amount)
        .set("from", deposit.from);
}
 

for withdrawal in events.withdrawals {
    tables
        .create_row("Withdrawal", &withdrawal.nullifier_hash) // Using nullifier_hash as unique ID
        .set("to", withdrawal.to)
        .set("relayer", withdrawal.relayer)
        .set("fee", withdrawal.fee)
        .set("blockNumber", withdrawal.block_number)
        .set("timestamp", withdrawal.block_time.unwrap().seconds);
}

Ok(tables.to_entity_changes())
}
// #[substreams::handlers::store]
// fn store_unique_identifiers(block: &Block, store: StoreSetString) {
//     // ... (Your existing store_unique_identifiers logic)
// }

// #[substreams::handlers::store]
// fn store_max_values(block: &Block, store: StoreMaxInt64) {
//     // ... (Your existing store_max_values logic)
// }


// fn map_tornado_metrics(
//     block: &Block,
//     additive_store: StoreGetInt64,
//     max_store: StoreGetInt64,
// ) -> Result<pb::tornado::DepositMetrics, Error> {
//     // ... (Your existing map_tornado_metrics logic)
// }