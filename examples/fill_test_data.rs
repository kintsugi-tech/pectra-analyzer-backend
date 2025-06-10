use pectralizer::{
    server::types::DailyBatcherStats,
    tracker::database::{Database, SqliteDatabase},
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Filling database with test data...");

    // Connect to the database
    let db = SqliteDatabase::new("./l2_batches_monitoring.db", 0).await?;

    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Exact addresses from L2_BATCHERS_ADDRESSES in l2_monitor.rs (lower-case)
    let base_batcher = "0x5050f69a9786f081509234f1a7f4684b5e5b76c9"; // Base
    let optimism_batcher = "0x6887246668a3b87f54deb3b94ba47a6f63f32985"; // Optimism

    // ---------------------------------------------------------------------
    // 1. Insert synthetic DAILY SNAPSHOT rows so the /seven_day_stats API
    //    immediately returns meaningful data without waiting for the
    //    background snapshot loop.
    // ---------------------------------------------------------------------

    let mut snapshot_rows: Vec<DailyBatcherStats> = Vec::new();

    let day_start_ts = (now / 86_400) * 86_400; // midnight UTC of current day

    for i in 1..=7 {
        let ts = day_start_ts - (i as i64) * 86_400; // midnight of previous days

        // helper to fabricate some deterministic numbers just to see variety
        let make_row = |addr: &str, factor: u64| DailyBatcherStats {
            batcher_address: addr.to_string(),
            snapshot_timestamp: ts,
            total_daily_txs: 100 + factor * i, // 100,110,... or 100,120,...
            total_eth_saved_wei: (1_000_000_000_000u128) * (i as u128) * (factor as u128),
            total_blob_data_gas: 1_000 * factor * i, // 1000,2000,...
            total_pectra_data_gas: 2_000 * factor * i, // 2000,4000,...
        };

        snapshot_rows.push(make_row(base_batcher, 1));
        snapshot_rows.push(make_row(optimism_batcher, 2));
    }

    println!(
        "📊 Inserting {} synthetic daily snapshot rows...",
        snapshot_rows.len()
    );
    db.insert_daily_batcher_stats(&snapshot_rows).await?;

    println!("\n🎉 Test snapshot insertion completed!");
    println!(
        "Inserted 7 days × 2 batchers = {} rows",
        snapshot_rows.len()
    );

    Ok(())
}
