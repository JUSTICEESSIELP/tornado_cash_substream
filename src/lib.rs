mod abi;
mod pb;
use crate::pb::tornado::types::v1::{Deposit, TornadoEvents, Withdrawal};
use hex_literal::hex;
use num_bigint::BigUint;
use std::ops::Mul;
use std::str::FromStr;
use substreams::errors::Error;
use substreams::scalar::{BigDecimal};
use substreams::store::StoreNew;
use substreams::store::{StoreAdd, StoreAddInt64, StoreGet, StoreGetBigDecimal, StoreGetInt64};
use substreams::Hex;
use substreams_entity_change::pb::entity::EntityChanges;
use substreams_entity_change::tables::Tables as EntityChangesTable;
use substreams_ethereum::pb::eth::v2 as eth;

// Contract address for the ZK pool
const TORNADO_POOL_ADDRESS: [u8; 20] = hex!("722122dF12D4e14e13Ac3b6895a86e84145b6967");
const PRECISION_FACTOR: u64 = 1_000_000_000_000_000_000_u64;


substreams_ethereum::init!();

#[substreams::handlers::map]
fn tornado_event_mapper(
    block: eth::Block,
    chainlink_price: StoreGetBigDecimal,
) -> Result<Option<TornadoEvents>, Error> {
    let mut tornado_events = TornadoEvents {
        deposits: vec![],
        withdrawals: vec![],
    };

    let pool_address = TORNADO_POOL_ADDRESS.to_vec();


    
    let eth_usd_rate = match chainlink_price.get_last("price_by_symbol:ETH:USD") {
        Some(price) => {
            substreams::log::info!("Found price: {}", price);
            price
        }
        None => {
            substreams::log::info!("No price found, using default 0");
            BigDecimal::from(0)
        }
    };
    for receipt_view in block.receipts() {
        let receipt = &receipt_view.receipt;
        let transaction = &receipt_view.transaction; // Get the transaction


        for log in &receipt.logs {
            if log.address.to_vec() == pool_address {
                if let Some(deposit_event) = abi::tornado_cash::events::Deposit::decode(log).ok() {
                    let amount = transaction.value.as_ref().map_or("0".to_string(), |v| {
                        let big_uint = BigUint::from_bytes_be(&v.bytes);
                        let amount_str = big_uint.to_string();

                        match amount_str.parse::<u64>() {
                            Ok(val) => {
                                let result = val / PRECISION_FACTOR; // Integer division
                                result.to_string()
                            }
                            Err(e) => {
                                substreams::log::info!(
                                    "Failed to parse amount: {} - Error: {}",
                                    amount_str,
                                    e
                                );
                                "0".to_string()
                            }
                        }
                    });

                    let convert_amount =
                        BigDecimal::from_str(&amount).unwrap_or(BigDecimal::from(0));
                    // Add debug logging
                    substreams::log::info!("Amount in ETH: {}", convert_amount);
                    substreams::log::info!("ETH/USD rate: {}", eth_usd_rate.clone());
                    let amount_usd = eth_usd_rate.clone().mul(convert_amount);
                    substreams::log::info!("Calculated USD amount: {}", amount_usd);

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
                        hash,
                        usdc_amount: amount_usd.to_string(),
                    });
                }

                if let Some(withdrawal_event) =
                    abi::tornado_cash::events::Withdrawal::decode(log).ok()
                {
                    let withdraw_amount_eth =
                        transaction.value.as_ref().map_or("0".to_string(), |v| {
                            let big_uint = BigUint::from_bytes_be(&v.bytes);
                            let amount_str = big_uint.to_string();

                            match amount_str.parse::<u64>() {
                                Ok(val) => {
                                    let result = val / PRECISION_FACTOR; // Integer division
                                    result.to_string()
                                }
                                Err(e) => {
                                    substreams::log::info!(
                                        "Failed to parse amount: {} - Error: {}",
                                        amount_str,
                                        e
                                    );
                                    "0".to_string()
                                }
                            }
                        });

                    let withdraw_convert_amount =
                        BigDecimal::from_str(&withdraw_amount_eth).unwrap_or(BigDecimal::from(0));
                    let withdraw_convert_amount_clone = withdraw_convert_amount.clone();
                    let withrawal_amount_usd = eth_usd_rate.clone().mul(withdraw_convert_amount_clone);

                    // Add debug logging

                    // let withdraw_amount_usdc =
                    let fee_str = withdrawal_event.fee.to_string();
                    let fee = match fee_str.parse::<u64>() {
                        Ok(f) => f,
                        Err(e) => {
                            substreams::log::info!(
                                "Failed to parse fee: {} - Error: {}",
                                fee_str,
                                e
                            );
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
                        fee: fee.to_string(),
                        block_number: block.number,
                        block_time: Some(block.timestamp().to_owned()),
                        log_index: log.block_index,
                        amount: withdraw_amount_eth,
                        usdc_amount: withrawal_amount_usd.to_string(),
                    });
                }
            }
        }
    }

    Ok(Some(tornado_events))
}

#[substreams::handlers::store]
pub fn store_additive_metrics(events: TornadoEvents, output: StoreAddInt64) {
    for deposit_event in events.deposits {
        if let Some(block_time) = deposit_event.block_time {
            let log_ordinal = deposit_event.log_index as u64;

            // Debug the incoming USDC amount
            substreams::log::info!("Raw USDC amount from deposit: {}", &deposit_event.usdc_amount);

            // Convert the decimal string to a value we can store
            let total_usdc_amount_deposited = match deposit_event.usdc_amount.split('.').next() {
                Some(whole_part) => {
                    match whole_part.parse::<i64>() {
                        Ok(amount) => {
                            substreams::log::info!("Successfully parsed deposit amount: {}", amount);
                            amount
                        },
                        Err(e) => {
                            substreams::log::info!("Failed to parse deposit amount: {}", e);
                            0_i64
                        }
                    }
                },
                None => 0_i64
            };

            // Log the value being stored
            substreams::log::info!(
                "Storing deposit - Ordinal: {}, Amount: {}", 
                log_ordinal,
                total_usdc_amount_deposited
            );

            // Store the value with explicit ordinal
            output.add(
                0,
                "total_deposits",
                total_usdc_amount_deposited,
            );

            
            let hour = block_time.seconds / 3600;
            output.add(
                0, 
                &format!("deposits_hour:{}", hour), 
                total_usdc_amount_deposited 
            );

            // Add daily metrics
            let day = block_time.seconds / 86400;
            output.add(
                0, 
                &format!("deposits_day:{}", day), 
                total_usdc_amount_deposited 
            );
        }
    }

   
    for withdraw_event in events.withdrawals {
        if let Some(block_time) = withdraw_event.block_time {
            let log_ordinal = withdraw_event.log_index as u64;

            substreams::log::info!("Raw USDC amount from withdrawal: {}", &withdraw_event.usdc_amount);

            let total_usdc_amount_withdraw = match withdraw_event.usdc_amount.split('.').next() {
                Some(whole_part) => {
                    match whole_part.parse::<i64>() {
                        Ok(amount) => {
                            substreams::log::info!("Successfully parsed withdrawal amount: {}", amount);
                            amount
                        },
                        Err(e) => {
                            substreams::log::info!("Failed to parse withdrawal amount: {}", e);
                            0_i64
                        }
                    }
                },
                None => 0_i64
            };

            substreams::log::info!(
                "Storing withdrawal - Ordinal: {}, Amount: {}", 
                log_ordinal,
                total_usdc_amount_withdraw
            );

            output.add(0, "total_withdrawals", total_usdc_amount_withdraw);

           
            let hour = block_time.seconds / 3600;
            let day = block_time.seconds / 86400;

            output.add(
                log_ordinal, 
                &format!("withdrawals_hour:{}", hour), 
                total_usdc_amount_withdraw
            );
            
            output.add(
                log_ordinal, 
                &format!("withdrawals_day:{}", day),
                total_usdc_amount_withdraw
            );

         
            if let Ok(fee) = withdraw_event.fee.parse::<i64>() {
                output.add(log_ordinal, "total_fees_to_withdraw", fee);
            }

           
            output.add(
                log_ordinal,
                &format!("relayer_withdrawals:{}", withdraw_event.relayer),
                1,
            );

            output.add(
                log_ordinal,
                &format!("recipient_withdrawals:{}", withdraw_event.to),
                1,
            );
        }
    }
}


#[substreams::handlers::map]
pub fn graph_out(
    events: TornadoEvents,
    store: StoreGetInt64,
) -> Result<EntityChanges, substreams::errors::Error> {
    // hash map of name to a table

    let mut tables  = EntityChangesTable::new();

       
    let total_deposits = store.get_last("total_deposits").unwrap_or_default();
    substreams::log::info!("Retrieved total_deposits from store: {}", total_deposits);
    
   
    let total_withdrawals = store.get_last("total_withdrawals").unwrap_or_default();
    substreams::log::info!("Retrieved total_withdrawals from store: {}", total_withdrawals);
    
  
 

    tables
    .create_row("PoolStats", "pool_stats")
    .set("totalDepositsInDollars", total_deposits)
    .set("totalWithdrawalsInDollars", total_withdrawals);
  



     

    for deposit in events.deposits {
        tables
            .create_row("Deposit", &deposit.hash) // Using hash as unique ID
            .set("commitment", deposit.commitment)
            .set("blockNumber", deposit.block_number)
            .set("timestamp", deposit.block_time.unwrap().seconds)
            .set("amount", deposit.amount)
            .set("from", deposit.from)
            .set("usdc_amount",deposit.usdc_amount);
    }

    for withdrawal in events.withdrawals {
        tables
            .create_row("Withdrawal", &withdrawal.nullifier_hash) // Using nullifier_hash as unique ID
            .set("to", withdrawal.to)
            .set("relayer", withdrawal.relayer)
            .set("fee", withdrawal.fee)
            .set("blockNumber", withdrawal.block_number)
            .set("timestamp", withdrawal.block_time.unwrap().seconds)
            .set("withdraw_eth_amount", withdrawal.amount)
            .set("withdraw_usdc_amount",withdrawal.usdc_amount);
    }



    Ok(tables.to_entity_changes())
}
