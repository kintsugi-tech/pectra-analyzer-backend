use crate::provider::ProviderState;
use alloy_primitives::{Address, hex::FromHex, FixedBytes};
use alloy_provider::Provider;
use rusqlite::Connection;
use serde_json;
use std::sync::{Arc, LazyLock};
use std::time::{SystemTime, UNIX_EPOCH};

// Placeholder for the L2 batcher addresses
static L2_BATCHERS_ADDRESSES: LazyLock<Vec<Address>> = LazyLock::new(|| {
    let mut addresses = vec![];
    addresses.push(Address::from_hex("0x5050F69a9786F081509234F1a7F4684b5E5b76C9").unwrap()); // Base
    addresses
});

/// Check if a transaction hash already exists in the database
fn is_tx_already_tracked(conn: &Connection, tx_hash: &str) -> eyre::Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM l2_batches_txs WHERE tx_hash = ?")?;
    let exists = stmt.exists([tx_hash])?;
    Ok(exists)
}

pub async fn start_monitoring(
    db_conn: Arc<Connection>,
    provider_state: ProviderState,
) -> eyre::Result<()> {
    println!("L2 Batches Monitoring Service: Initializing...");

    loop {
        println!(
            "L2 Batches Monitoring Service: Starting hourly check for new transactions. Monitored addresses: {:?}",
            L2_BATCHERS_ADDRESSES.iter().map(|a| format!("{:#x}", a)).collect::<Vec<_>>()
        );

        let start_block = crate::tracker::db::get_last_analyzed_block(&db_conn)?;
        let current_block = provider_state.ethereum_provider.get_block_number().await?;
        
        println!("Checking transactions from block {} to {}", start_block, current_block);

        // for each monitored address, get its transactions
        for &batcher_address in L2_BATCHERS_ADDRESSES.iter() {
            println!("Checking transactions for batcher address: {:#x}", batcher_address);
            
            // get (up to 200) normal transactions from Etherscan
            match provider_state.etherscan_provider.get_normal_txs(
                batcher_address,
                1, // mainnet chain ID
                start_block,
                current_block,
                200
            ).await {
                Ok(response) => {
                    println!("Found {} transactions for address {:#x}", response.result.len(), batcher_address);
                    
                    for tx in response.result {
                        let tx_hash = format!("{:#x}", tx.hash);
                        
                        if is_tx_already_tracked(&db_conn, &tx_hash)? {
                            println!("Skipping already tracked transaction: {}", tx_hash);
                            continue;
                        }

                        println!("Processing new transaction: {}", tx_hash);
                        
                        // Analyze the transaction using provider_state
                        let tx_hash_bytes = FixedBytes::from_hex(&tx_hash)
                            .map_err(|e| eyre::eyre!("Failed to parse transaction hash: {}", e))?;
                        
                        let analysis_result = match crate::server::handlers::analyze_transaction(&provider_state, tx_hash_bytes).await {
                            Ok(analysis) => serde_json::to_string(&analysis)
                                .map_err(|e| eyre::eyre!("Failed to serialize analysis result: {}", e))?,
                            Err(e) => {
                                eprintln!("Failed to analyze transaction {}: {}. Skipping...", tx_hash, e);
                                continue;
                            }
                        };
                        
                        let tracked_batch = crate::tracker::db::TrackedBatch {
                            id: None,
                            tx_hash,
                            batcher_address: format!("{:#x}", batcher_address),
                            analysis_result,
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            last_analyzed_block: None,
                        };

                        // Save to database
                        if let Err(e) = crate::tracker::db::save_tracked_batch(&db_conn, &tracked_batch) {
                            eprintln!("Failed to save transaction {}: {}", tracked_batch.tx_hash, e);
                        } else {
                            println!("Successfully saved transaction: {}", tracked_batch.tx_hash);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching transactions for address {:#x}: {}", batcher_address, e);
                }
            }
        }

        // Update the last analyzed block
        if let Err(e) = crate::tracker::db::update_last_analyzed_block(&db_conn, current_block) {
            eprintln!("Failed to update last analyzed block: {}", e);
        }

        println!("L2 Batches Monitoring Service: Completed hourly check. Sleeping for 1 hour...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
