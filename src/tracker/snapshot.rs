use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use eyre::Result;
use tracing::{error, info};

use crate::tracker::database::Database;
use crate::server::types::{BatcherDailyTxs, BatcherEthSaved, BatcherBlobDataGas, BatcherPectraDataGas, DailyBatcherStats};

/// Start an infinite loop that creates and persists a daily snapshot of batcher metrics every 24 hours.
///
/// The snapshot aggregates data for the **previous** 24 hours for each batcher and stores the
/// results in the `daily_batcher_stats` table.
pub async fn start_snapshot_loop(db: Arc<dyn Database>) -> Result<()> {
    // Run an initial snapshot immediately so that the service starts with up-to-date data.
    if let Err(e) = create_and_save_snapshot(db.clone()).await {
        error!(?e, "Failed to create initial daily snapshot");
    }

    // Then run once every 24 h.
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60 * 24));
    loop {
        interval.tick().await;
        if let Err(e) = create_and_save_snapshot(db.clone()).await {
            error!(?e, "Failed to create daily snapshot");
        }
    }
}

async fn create_and_save_snapshot(db: Arc<dyn Database>) -> Result<()> {
    let end_ts = Utc::now().timestamp();
    let start_ts = end_ts - 60 * 60 * 24;

    // Aggregate metrics for all batchers.
    let daily_txs: Vec<BatcherDailyTxs> = db.get_all_daily_transactions(start_ts, end_ts).await?;
    let eth_saved: Vec<BatcherEthSaved> = db.get_all_eth_saved_data(start_ts, end_ts).await?;
    let blob_gas: Vec<BatcherBlobDataGas> = db.get_all_total_blob_data_gas(start_ts, end_ts).await?;
    let pectra_gas: Vec<BatcherPectraDataGas> = db.get_all_total_pectra_data_gas(start_ts, end_ts).await?;

    #[derive(Default)]
    struct TmpStats {
        total_daily_txs: u64,
        total_eth_saved_wei: u128,
        total_blob_data_gas: u64,
        total_pectra_data_gas: u64,
    }

    let mut map: HashMap<String, TmpStats> = HashMap::new();

    for item in daily_txs {
        map.entry(item.batcher_address)
            .or_default()
            .total_daily_txs = item.tx_count;
    }
    for item in eth_saved {
        map.entry(item.batcher_address)
            .or_default()
            .total_eth_saved_wei = item.total_eth_saved_wei;
    }
    for item in blob_gas {
        map.entry(item.batcher_address)
            .or_default()
            .total_blob_data_gas = item.total_blob_data_gas;
    }
    for item in pectra_gas {
        map.entry(item.batcher_address)
            .or_default()
            .total_pectra_data_gas = item.total_pectra_data_gas;
    }

    let snapshot_ts = end_ts;

    let mut stats_vec = Vec::with_capacity(map.len());
    for (batcher_address, s) in map {
        stats_vec.push(DailyBatcherStats {
            batcher_address,
            snapshot_timestamp: snapshot_ts,
            total_eth_saved_wei: s.total_eth_saved_wei,
            total_daily_txs: s.total_daily_txs,
            total_blob_data_gas: s.total_blob_data_gas,
            total_pectra_data_gas: s.total_pectra_data_gas,
        });
    }

    db.insert_daily_batcher_stats(&stats_vec).await?;
    info!(count = stats_vec.len(), "Daily batcher snapshot saved");

    Ok(())
} 