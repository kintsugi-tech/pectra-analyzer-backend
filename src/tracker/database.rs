use async_trait::async_trait;
use eyre::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

// This struct might need to be a bit more generic or defined outside if multiple DBs have different needs
// For now, keeping it as is for simplicity of refactoring existing code.
#[derive(Debug, Clone, sqlx::FromRow)] // Added sqlx::FromRow for query_as potential
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

#[async_trait]
pub trait Database: Send + Sync {
    async fn is_tx_already_tracked(&self, tx_hash: &str) -> Result<bool>;
    async fn save_tracked_batch(&self, batch: &TrackedBatch) -> Result<()>;
    async fn get_last_analyzed_block(&self) -> Result<u64>;
    async fn update_last_analyzed_block(&self, block_number: u64) -> Result<()>;
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
        .bind(&batch.batcher_address)
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
}
