use crate::server::types::{
    BatcherBlobDataGas, BatcherDailyTxs, BatcherEthSaved, BatcherPectraDataGas,
};
use async_trait::async_trait;
use eyre::Result;
use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackedBatch {
    // sqlx::FromRow requires fields to match column names or use #[sqlx(rename = "...")]
    // Assuming id is not always fetched or used directly in inserts by this struct, keeping Option.
    // If id were from DB, it would typically not be Option<i64> for FromRow unless nullable.
    // However, our INSERTs don't use ID, and SELECTs might not always fetch it.
    // For simplicity, the struct stays as is; specific queries will manage what they select/insert.
    pub id: Option<i64>,
    pub tx_hash: String,
    pub batcher_address: String,
    pub analysis_result: String,
    pub timestamp: i64, // SQLite INTEGER can be mapped to i64
    #[sqlx(default)] // If last_analyzed_block is not selected, it will default.
    pub last_analyzed_block: Option<i64>, // SQLite INTEGER can be Option<i64>
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FailedTransaction {
    pub id: Option<i64>,
    pub tx_hash: String,
    pub batcher_address: String,
    pub error_message: String,
    pub retry_count: i32,
    pub next_retry_at: i64,     // Unix timestamp
    pub first_failed_at: i64,   // Unix timestamp
    pub last_attempted_at: i64, // Unix timestamp
}

#[async_trait]
pub trait Database: Send + Sync {
    async fn is_tx_already_tracked(&self, tx_hash: &str) -> Result<bool>;
    async fn save_tracked_batch(&self, batch: &TrackedBatch) -> Result<()>;
    async fn get_last_analyzed_block(&self) -> Result<u64>;
    async fn update_last_analyzed_block(&self, block_number: u64) -> Result<()>;

    // methods for failed transaction handling
    async fn save_failed_transaction(&self, failed_tx: &FailedTransaction) -> Result<()>;
    async fn get_failed_transactions_ready_for_retry(&self) -> Result<Vec<FailedTransaction>>;
    async fn update_failed_transaction_retry(
        &self,
        tx_hash: &str,
        retry_count: i32,
        next_retry_at: i64,
        error_message: &str,
    ) -> Result<()>;
    async fn remove_failed_transaction(&self, tx_hash: &str) -> Result<()>;
    async fn is_tx_in_failed_queue(&self, tx_hash: &str) -> Result<bool>;

    // methods for L2 batch analytics
    async fn get_daily_transactions(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64>; // tx_count for specific batcher

    async fn get_eth_saved_data(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u128>; // total eth_saved_wei for specific batcher

    async fn get_total_blob_data_gas(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64>;

    async fn get_total_pectra_data_gas(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64>;

    // methods for aggregated L2 batch analytics across all batchers
    async fn get_all_daily_transactions(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherDailyTxs>>;

    async fn get_all_eth_saved_data(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherEthSaved>>;

    async fn get_all_total_blob_data_gas(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherBlobDataGas>>;

    async fn get_all_total_pectra_data_gas(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherPectraDataGas>>;
}

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new(db_path: &str, initial_block: u64) -> Result<Self> {
        // ensure the db file can be created by sqlx, e.g. by ensuring parent directory exists.
        // sqlx creates the file if it doesn't exist with mode=rwc.
        let db_url = format!("sqlite://{}?mode=rwc", db_path);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        // create l2 batches txs table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS l2_batches_txs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_hash TEXT NOT NULL UNIQUE,
                batcher_address TEXT NOT NULL,
                analysis_result TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                last_analyzed_block INTEGER
            )",
        )
        .execute(&pool)
        .await?;

        // create failed transactions table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS failed_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_hash TEXT NOT NULL UNIQUE,
                batcher_address TEXT NOT NULL,
                error_message TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                next_retry_at INTEGER NOT NULL,
                first_failed_at INTEGER NOT NULL,
                last_attempted_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        // sqlx uses `?` for SQLite parameters, not `?1` etc. for numbered params by default.
        // But for `VALUES (...)` it can be `VALUES (?, ?, ...)`
        let initial_block_i64 = initial_block as i64;
        sqlx::query(
            "INSERT OR IGNORE INTO l2_batches_txs (tx_hash, batcher_address, analysis_result, timestamp, last_analyzed_block)
             VALUES ('monitoring_state', 'monitoring_state', '{}', 0, ?)",
        )
        .bind(initial_block_i64)
        .execute(&pool)
        .await?;

        Ok(SqliteDatabase { pool })
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn is_tx_already_tracked(&self, tx_hash: &str) -> Result<bool> {
        let result =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM l2_batches_txs WHERE tx_hash = ?")
                .bind(tx_hash)
                .fetch_one(&self.pool)
                .await?;
        Ok(result > 0)
    }

    async fn save_tracked_batch(&self, batch: &TrackedBatch) -> Result<()> {
        sqlx::query(
            "INSERT INTO l2_batches_txs (tx_hash, batcher_address, analysis_result, timestamp, last_analyzed_block)
             VALUES (?, ?, ?, ?, NULL)", // last_analyzed_block is NULL for normal txs
        )
        .bind(&batch.tx_hash)
        .bind(batch.batcher_address.to_lowercase()) // Store addresses in lowercase for consistency
        .bind(&batch.analysis_result)
        .bind(batch.timestamp) // sqlx can map i64 to INTEGER
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_last_analyzed_block(&self) -> Result<u64> {
        let block_i64 = sqlx::query_scalar::<_, i64>(
            "SELECT last_analyzed_block FROM l2_batches_txs WHERE tx_hash = 'monitoring_state'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(block_i64 as u64)
    }

    async fn update_last_analyzed_block(&self, block_number: u64) -> Result<()> {
        let block_number_i64 = block_number as i64;
        sqlx::query(
            "UPDATE l2_batches_txs SET last_analyzed_block = ? WHERE tx_hash = 'monitoring_state'",
        )
        .bind(block_number_i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn save_failed_transaction(&self, failed_tx: &FailedTransaction) -> Result<()> {
        sqlx::query(
            "INSERT INTO failed_transactions (tx_hash, batcher_address, error_message, retry_count, next_retry_at, first_failed_at, last_attempted_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&failed_tx.tx_hash)
        .bind(failed_tx.batcher_address.to_lowercase()) // Store addresses in lowercase for consistency
        .bind(&failed_tx.error_message)
        .bind(failed_tx.retry_count)
        .bind(failed_tx.next_retry_at)
        .bind(failed_tx.first_failed_at)
        .bind(failed_tx.last_attempted_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_failed_transactions_ready_for_retry(&self) -> Result<Vec<FailedTransaction>> {
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let transactions = sqlx::query_as::<_, FailedTransaction>(
            "SELECT id, tx_hash, batcher_address, error_message, retry_count, next_retry_at, first_failed_at, last_attempted_at
             FROM failed_transactions
             WHERE next_retry_at <= ?
             ORDER BY next_retry_at"
        )
        .bind(current_timestamp)
        .fetch_all(&self.pool)
        .await?;
        Ok(transactions)
    }

    async fn update_failed_transaction_retry(
        &self,
        tx_hash: &str,
        retry_count: i32,
        next_retry_at: i64,
        error_message: &str,
    ) -> Result<()> {
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query(
            "UPDATE failed_transactions SET retry_count = ?, next_retry_at = ?, error_message = ?, last_attempted_at = ?
             WHERE tx_hash = ?",
        )
        .bind(retry_count)
        .bind(next_retry_at)
        .bind(error_message)
        .bind(current_timestamp)
        .bind(tx_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_failed_transaction(&self, tx_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM failed_transactions WHERE tx_hash = ?")
            .bind(tx_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn is_tx_in_failed_queue(&self, tx_hash: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM failed_transactions WHERE tx_hash = ?",
        )
        .bind(tx_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(result > 0)
    }

    async fn get_daily_transactions(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM l2_batches_txs 
             WHERE batcher_address = LOWER(?) AND timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(batcher_address)
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    async fn get_eth_saved_data(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u128> {
        let rows = sqlx::query(
            "SELECT analysis_result FROM l2_batches_txs 
             WHERE batcher_address = LOWER(?) AND timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(batcher_address)
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut total_eth_saved = 0u128;
        for row in rows {
            let analysis_result: String = row.get("analysis_result");

            // Parse the JSON analysis result to extract ETH saved data
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let blob_data_wei_spent =
                    analysis["blob_data_wei_spent"].as_u64().unwrap_or(0) as u128;
                let eip_7623_calldata_wei_spent = analysis["eip_7623_calldata_wei_spent"]
                    .as_u64()
                    .unwrap_or(0) as u128;

                // Calculate ETH saved: difference between what would be spent on EIP-7623 and what was actually spent on blob data
                let eth_saved_wei = eip_7623_calldata_wei_spent.saturating_sub(blob_data_wei_spent);
                total_eth_saved += eth_saved_wei;
            }
        }

        Ok(total_eth_saved)
    }

    async fn get_total_blob_data_gas(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64> {
        let rows = sqlx::query(
            "SELECT analysis_result FROM l2_batches_txs 
             WHERE batcher_address = LOWER(?) AND timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(batcher_address)
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut total_blob_gas = 0u64;
        for row in rows {
            let analysis_result: String = row.get("analysis_result");
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let blob_gas_used = analysis["blob_gas_used"].as_u64().unwrap_or(0);
                total_blob_gas += blob_gas_used;
            }
        }

        Ok(total_blob_gas)
    }

    async fn get_total_pectra_data_gas(
        &self,
        batcher_address: &str,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<u64> {
        let rows = sqlx::query(
            "SELECT analysis_result FROM l2_batches_txs 
             WHERE batcher_address = LOWER(?) AND timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(batcher_address)
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut total_pectra_gas = 0u64;
        for row in rows {
            let analysis_result: String = row.get("analysis_result");
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let eip_7623_calldata_gas = analysis["eip_7623_calldata_gas"].as_u64().unwrap_or(0);
                total_pectra_gas += eip_7623_calldata_gas;
            }
        }

        Ok(total_pectra_gas)
    }

    async fn get_all_daily_transactions(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherDailyTxs>> {
        let rows = sqlx::query(
            "SELECT batcher_address, COUNT(*) FROM l2_batches_txs 
             WHERE timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'
             GROUP BY batcher_address",
        )
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut all_daily_transactions = Vec::new();
        for row in rows {
            let batcher_address: String = row.get("batcher_address");
            let tx_count: i64 = row.get("COUNT(*)");
            all_daily_transactions.push(BatcherDailyTxs {
                batcher_address,
                tx_count: tx_count as u64,
            });
        }

        Ok(all_daily_transactions)
    }

    async fn get_all_eth_saved_data(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherEthSaved>> {
        let rows = sqlx::query(
            "SELECT batcher_address, analysis_result FROM l2_batches_txs 
             WHERE timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut batcher_eth_saved: std::collections::HashMap<String, u128> =
            std::collections::HashMap::new();

        for row in rows {
            let batcher_address: String = row.get("batcher_address");
            let analysis_result: String = row.get("analysis_result");

            // Parse the JSON analysis result to extract ETH saved data
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let blob_data_wei_spent =
                    analysis["blob_data_wei_spent"].as_u64().unwrap_or(0) as u128;
                let eip_7623_calldata_wei_spent = analysis["eip_7623_calldata_wei_spent"]
                    .as_u64()
                    .unwrap_or(0) as u128;

                // Calculate ETH saved: difference between what would be spent on EIP-7623 and what was actually spent on blob data
                let eth_saved_wei = eip_7623_calldata_wei_spent.saturating_sub(blob_data_wei_spent);

                *batcher_eth_saved.entry(batcher_address).or_insert(0) += eth_saved_wei;
            }
        }

        Ok(batcher_eth_saved
            .into_iter()
            .map(|(batcher_address, total_eth_saved_wei)| BatcherEthSaved {
                batcher_address,
                total_eth_saved_wei,
            })
            .collect())
    }

    async fn get_all_total_blob_data_gas(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherBlobDataGas>> {
        let rows = sqlx::query(
            "SELECT batcher_address, analysis_result FROM l2_batches_txs 
             WHERE timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut batcher_blob_gas: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();

        for row in rows {
            let batcher_address: String = row.get("batcher_address");
            let analysis_result: String = row.get("analysis_result");
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let blob_gas_used = analysis["blob_gas_used"].as_u64().unwrap_or(0);
                *batcher_blob_gas.entry(batcher_address).or_insert(0) += blob_gas_used;
            }
        }

        Ok(batcher_blob_gas
            .into_iter()
            .map(
                |(batcher_address, total_blob_data_gas)| BatcherBlobDataGas {
                    batcher_address,
                    total_blob_data_gas,
                },
            )
            .collect())
    }

    async fn get_all_total_pectra_data_gas(
        &self,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<BatcherPectraDataGas>> {
        let rows = sqlx::query(
            "SELECT batcher_address, analysis_result FROM l2_batches_txs 
             WHERE timestamp >= ? AND timestamp <= ? 
             AND tx_hash != 'monitoring_state'",
        )
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&self.pool)
        .await?;

        let mut batcher_pectra_gas: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();

        for row in rows {
            let batcher_address: String = row.get("batcher_address");
            let analysis_result: String = row.get("analysis_result");
            if let Ok(analysis) = serde_json::from_str::<serde_json::Value>(&analysis_result) {
                let eip_7623_calldata_gas = analysis["eip_7623_calldata_gas"].as_u64().unwrap_or(0);
                *batcher_pectra_gas.entry(batcher_address).or_insert(0) += eip_7623_calldata_gas;
            }
        }

        Ok(batcher_pectra_gas
            .into_iter()
            .map(
                |(batcher_address, total_pectra_data_gas)| BatcherPectraDataGas {
                    batcher_address,
                    total_pectra_data_gas,
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::NamedTempFile;

    async fn create_test_database() -> Result<SqliteDatabase> {
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap();
        let db = SqliteDatabase::new(db_path, 0).await?;

        // prevent the temp file from being deleted
        std::mem::forget(temp_file);

        Ok(db)
    }

    #[tokio::test]
    async fn test_case_insensitive_batcher_address_search() -> Result<()> {
        let db = create_test_database().await?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // create test data with mixed case address
        let mixed_case_address = "0x5050F69a9786F081509234F1a7F4684b5E5b76C9"; // Mixed case
        let batch = TrackedBatch {
            id: None,
            tx_hash: "0xtest123".to_string(),
            batcher_address: mixed_case_address.to_string(),
            analysis_result: r#"{"blob_gas_used": 100000, "eip_7623_calldata_gas": 5000, "blob_data_wei_spent": 2000000000000000, "eip_7623_calldata_wei_spent": 3000000000000000}"#.to_string(),
            timestamp: now,
            last_analyzed_block: None,
        };

        // save the batch (should be stored in lowercase)
        db.save_tracked_batch(&batch).await?;

        // test searches with different cases
        let test_cases = vec![
            "0x5050f69a9786f081509234f1a7f4684b5e5b76c9", // all lowercase
            "0x5050F69A9786F081509234F1A7F4684B5E5B76C9", // all uppercase
            "0x5050F69a9786F081509234F1a7F4684b5E5b76C9", // mixed case (original)
        ];

        for test_address in test_cases {
            println!("Testing address: {}", test_address);

            // test get_daily_transactions
            let count = db
                .get_daily_transactions(test_address, now - 100, now + 100)
                .await?;
            assert_eq!(
                count, 1,
                "get_daily_transactions failed for address: {}",
                test_address
            );

            // test get_total_blob_data_gas
            let blob_gas = db
                .get_total_blob_data_gas(test_address, now - 100, now + 100)
                .await?;
            assert_eq!(
                blob_gas, 100000,
                "get_total_blob_data_gas failed for address: {}",
                test_address
            );

            // test get_total_pectra_data_gas
            let pectra_gas = db
                .get_total_pectra_data_gas(test_address, now - 100, now + 100)
                .await?;
            assert_eq!(
                pectra_gas, 5000,
                "get_total_pectra_data_gas failed for address: {}",
                test_address
            );

            // test get_eth_saved_data
            let eth_saved = db
                .get_eth_saved_data(test_address, now - 100, now + 100)
                .await?;
            assert_eq!(
                eth_saved, 1000000000000000,
                "get_eth_saved_data failed for address: {}",
                test_address
            ); // 3000000000000000 - 2000000000000000
        }

        println!("✅ All case insensitive tests passed!");
        Ok(())
    }

    #[tokio::test]
    async fn test_batcher_address_stored_as_lowercase() -> Result<()> {
        let db = create_test_database().await?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // create test data with mixed case address
        let mixed_case_address = "0x5050F69a9786F081509234F1a7F4684b5E5b76C9";
        let batch = TrackedBatch {
            id: None,
            tx_hash: "0xtest456".to_string(),
            batcher_address: mixed_case_address.to_string(),
            analysis_result: r#"{"blob_gas_used": 50000}"#.to_string(),
            timestamp: now,
            last_analyzed_block: None,
        };

        // save the batch
        db.save_tracked_batch(&batch).await?;

        // verify that the address is stored in lowercase by checking the raw database
        let stored_address = sqlx::query_scalar::<_, String>(
            "SELECT batcher_address FROM l2_batches_txs WHERE tx_hash = ?",
        )
        .bind("0xtest456")
        .fetch_one(&db.pool)
        .await?;

        assert_eq!(stored_address, "0x5050f69a9786f081509234f1a7f4684b5e5b76c9");
        println!("✅ Address stored in lowercase as expected!");

        Ok(())
    }
}
