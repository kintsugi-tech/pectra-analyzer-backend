use crate::provider::ProviderState;
use crate::tracker::database::{Database, FailedTransaction, TrackedBatch};
use alloy_primitives::{FixedBytes, hex::FromHex};
use eyre::Result;
use serde_json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Maximum number of retry attempts before giving up
const MAX_RETRY_ATTEMPTS: i32 = 5;

/// Base delay in seconds for exponential backoff
const BASE_RETRY_DELAY: u64 = 60; // 1 minute

/// Maximum delay in seconds to prevent extremely long waits
const MAX_RETRY_DELAY: u64 = 3600; // 1 hour

pub struct RetryHandler {
    db: Arc<dyn Database>,
    provider_state: ProviderState,
}

impl RetryHandler {
    pub fn new(db: Arc<dyn Database>, provider_state: ProviderState) -> Self {
        Self { db, provider_state }
    }

    /// Calculate the next retry time using exponential backoff
    fn calculate_next_retry_time(retry_count: i32) -> i64 {
        let delay = BASE_RETRY_DELAY * 2_u64.pow(retry_count as u32);
        let capped_delay = delay.min(MAX_RETRY_DELAY);

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        current_time + capped_delay as i64
    }

    /// Save a failed transaction to the retry queue
    pub async fn save_failed_transaction(
        &self,
        tx_hash: &str,
        batcher_address: &str,
        error_message: &str,
    ) -> Result<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // check if transaction is already in failed queue
        if self.db.is_tx_in_failed_queue(tx_hash).await? {
            // update existing failed transaction
            let next_retry_at = Self::calculate_next_retry_time(1);
            self.db
                .update_failed_transaction_retry(tx_hash, 1, next_retry_at, error_message)
                .await?;
            info!(
                "Updated existing failed transaction in retry queue: {}",
                tx_hash
            );
        } else {
            // create new failed transaction entry
            let failed_tx = FailedTransaction {
                id: None,
                tx_hash: tx_hash.to_string(),
                batcher_address: batcher_address.to_string(),
                error_message: error_message.to_string(),
                retry_count: 0,
                next_retry_at: Self::calculate_next_retry_time(0),
                first_failed_at: current_time,
                last_attempted_at: current_time,
            };

            self.db.save_failed_transaction(&failed_tx).await?;
            info!("Added new failed transaction to retry queue: {}", tx_hash);
        }

        Ok(())
    }

    /// Process failed transactions that are ready for retry
    pub async fn process_retry_queue(&self) -> Result<()> {
        let failed_transactions = self.db.get_failed_transactions_ready_for_retry().await?;

        if failed_transactions.is_empty() {
            return Ok(());
        }

        info!(
            "Processing {} failed transactions for retry",
            failed_transactions.len()
        );

        for failed_tx in failed_transactions {
            if failed_tx.retry_count >= MAX_RETRY_ATTEMPTS {
                warn!(
                    "Transaction {} has exceeded maximum retry attempts ({}), removing from queue",
                    failed_tx.tx_hash, MAX_RETRY_ATTEMPTS
                );
                if let Err(e) = self.db.remove_failed_transaction(&failed_tx.tx_hash).await {
                    error!("Failed to remove transaction from retry queue: {}", e);
                }
                continue;
            }

            info!(
                "Retrying transaction {} (attempt {}/{})",
                failed_tx.tx_hash,
                failed_tx.retry_count + 1,
                MAX_RETRY_ATTEMPTS
            );

            match self.retry_transaction_analysis(&failed_tx).await {
                Ok(analysis_result) => {
                    // Success! Save to main database and remove from retry queue
                    let tracked_batch = TrackedBatch {
                        id: None,
                        tx_hash: failed_tx.tx_hash.clone(),
                        batcher_address: failed_tx.batcher_address.clone(),
                        analysis_result,
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                        last_analyzed_block: None,
                    };

                    if let Err(e) = self.db.save_tracked_batch(&tracked_batch).await {
                        error!("Failed to save successfully retried transaction: {}", e);
                        // update retry info for next attempt
                        let next_retry_count = failed_tx.retry_count + 1;
                        let next_retry_at = Self::calculate_next_retry_time(next_retry_count);
                        if let Err(e) = self
                            .db
                            .update_failed_transaction_retry(
                                &failed_tx.tx_hash,
                                next_retry_count,
                                next_retry_at,
                                &format!("Database save error: {}", e),
                            )
                            .await
                        {
                            error!("Failed to update retry info: {}", e);
                        }
                    } else {
                        // successfully saved, remove from retry queue
                        if let Err(e) = self.db.remove_failed_transaction(&failed_tx.tx_hash).await
                        {
                            error!(
                                "Failed to remove successfully processed transaction from retry queue: {}",
                                e
                            );
                        } else {
                            info!(
                                "Successfully processed and saved transaction: {}",
                                failed_tx.tx_hash
                            );
                        }
                    }
                }
                Err(e) => {
                    // still failing, update retry info
                    let next_retry_count = failed_tx.retry_count + 1;
                    let next_retry_at = Self::calculate_next_retry_time(next_retry_count);

                    if let Err(update_err) = self
                        .db
                        .update_failed_transaction_retry(
                            &failed_tx.tx_hash,
                            next_retry_count,
                            next_retry_at,
                            &e.to_string(),
                        )
                        .await
                    {
                        error!(
                            "Failed to update retry info for {}: {}",
                            failed_tx.tx_hash, update_err
                        );
                    } else {
                        info!(
                            "Transaction {} still failing, scheduled for retry at {} (attempt {}/{})",
                            failed_tx.tx_hash,
                            chrono::DateTime::from_timestamp(next_retry_at, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                            next_retry_count + 1,
                            MAX_RETRY_ATTEMPTS
                        );
                    }
                }
            }

            // small delay between retries to avoid overwhelming the API
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(())
    }

    /// Retry analyzing a specific transaction
    async fn retry_transaction_analysis(&self, failed_tx: &FailedTransaction) -> Result<String> {
        let tx_hash_bytes = FixedBytes::from_hex(&failed_tx.tx_hash)
            .map_err(|e| eyre::eyre!("Failed to parse transaction hash: {}", e))?;

        let analysis_result =
            crate::server::handlers::analyze_transaction(&self.provider_state, tx_hash_bytes)
                .await?;

        serde_json::to_string(&analysis_result)
            .map_err(|e| eyre::eyre!("Failed to serialize analysis result: {}", e))
    }

    /// Start the retry processing loop
    pub async fn start_retry_loop(&self) -> Result<()> {
        info!("Starting failed transaction retry loop...");

        loop {
            if let Err(e) = self.process_retry_queue().await {
                error!("Error processing retry queue: {}", e);
            }

            // check for retries every 30 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}
