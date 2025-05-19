use rusqlite::{Connection, Result};

pub struct TrackedBatch {
    pub id: Option<i64>,
    pub tx_hash: String,
    pub batcher_address: String,
    pub analysis_result: String, // For simplicity, store analysis as JSON string or similar
    pub timestamp: i64,
    pub last_analyzed_block: Option<u64>, // None for transactions, Some for monitoring state
}

pub fn initialize_db(db_path: &str, initial_block: u64) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS l2_batches_txs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_hash TEXT NOT NULL UNIQUE,
            batcher_address TEXT NOT NULL,
            analysis_result TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            last_analyzed_block INTEGER
        )",
        [],
    )?;

    // initialize the monitoring state if it doesn't exist
    conn.execute(
        "INSERT OR IGNORE INTO l2_batches_txs (tx_hash, batcher_address, analysis_result, timestamp, last_analyzed_block)
         VALUES ('monitoring_state', 'monitoring_state', '{}', 0, ?1)",
        [initial_block],
    )?;

    Ok(conn)
}

pub fn save_tracked_batch(conn: &Connection, batch: &TrackedBatch) -> Result<()> {
    conn.execute(
        "INSERT INTO l2_batches_txs (tx_hash, batcher_address, analysis_result, timestamp, last_analyzed_block)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        (
            &batch.tx_hash,
            &batch.batcher_address,
            &batch.analysis_result,
            &batch.timestamp,
        ),
    )?;
    Ok(())
}

pub fn get_last_analyzed_block(conn: &Connection) -> Result<u64> {
    let mut stmt = conn.prepare("SELECT last_analyzed_block FROM l2_batches_txs WHERE tx_hash = 'monitoring_state'")?;
    let block: u64 = stmt.query_row([], |row| row.get(0))?;
    Ok(block)
}

pub fn update_last_analyzed_block(conn: &Connection, block_number: u64) -> Result<()> {
    conn.execute(
        "UPDATE l2_batches_txs SET last_analyzed_block = ?1 WHERE tx_hash = 'monitoring_state'",
        [block_number],
    )?;
    Ok(())
}
