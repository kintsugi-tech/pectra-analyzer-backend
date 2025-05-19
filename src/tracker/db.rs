use rusqlite::{Connection, Result};

pub struct TrackedBatch {
    pub id: Option<i64>,
    pub tx_hash: String,
    pub batcher_address: String,
    pub analysis_result: String, // For simplicity, store analysis as JSON string or similar
    pub timestamp: i64,
}

pub fn initialize_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS l2_batches_txs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_hash TEXT NOT NULL UNIQUE,
            batcher_address TEXT NOT NULL,
            analysis_result TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn save_tracked_batch(conn: &Connection, batch: &TrackedBatch) -> Result<()> {
    conn.execute(
        "INSERT INTO l2_batches_txs (tx_hash, batcher_address, analysis_result, timestamp)
         VALUES (?1, ?2, ?3, ?4)",
        (
            &batch.tx_hash,
            &batch.batcher_address,
            &batch.analysis_result,
            &batch.timestamp,
        ),
    )?;
    Ok(())
}
