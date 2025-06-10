use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use eyre::Result;
use tracing::{error, info};

use crate::server::types::{
    BatcherBlobDataGas, BatcherDailyTxs, BatcherEthSaved, BatcherPectraDataGas, DailyBatcherStats,
};
use crate::tracker::database::Database;

/// Start an infinite loop that creates and persists a daily snapshot of batcher metrics every 24 hours.
///
/// The snapshot aggregates data for the **previous** 24 hours for each batcher and stores the
/// results in the `daily_batcher_stats` table.
pub async fn start_snapshot_loop(db: Arc<dyn Database>) -> Result<()> {
    // run an initial snapshot immediately so that the service starts with up-to-date data.
    if let Err(e) = create_and_save_snapshot(db.clone()).await {
        error!(?e, "Failed to create initial daily snapshot");
    }

    // then run once every 24 h.
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60 * 24));
    loop {
        interval.tick().await;
        if let Err(e) = create_and_save_snapshot(db.clone()).await {
            error!(?e, "Failed to create daily snapshot");
        }
    }
}

async fn create_and_save_snapshot(db: Arc<dyn Database>) -> Result<()> {
    // align to whole-day boundaries (UTC) so that restarts within 24 h don't change the key
    // we always create the snapshot for the PREVIOUS day: [day_start, day_start + 86400)
    let now_ts = Utc::now().timestamp();
    let day_start_ts = (now_ts / 86_400) * 86_400; // midnight of current day UTC
    let start_ts = day_start_ts - 86_400; // midnight of previous day UTC
    let end_ts = day_start_ts - 1; // inclusive upper bound (23:59:59 of previous day)

    // aggregate metrics for all batchers
    let daily_txs: Vec<BatcherDailyTxs> = db.get_all_daily_transactions(start_ts, end_ts).await?;
    let eth_saved: Vec<BatcherEthSaved> = db.get_all_eth_saved_data(start_ts, end_ts).await?;
    let blob_gas: Vec<BatcherBlobDataGas> =
        db.get_all_total_blob_data_gas(start_ts, end_ts).await?;
    let pectra_gas: Vec<BatcherPectraDataGas> =
        db.get_all_total_pectra_data_gas(start_ts, end_ts).await?;

    #[derive(Default)]
    struct TmpStats {
        total_daily_txs: u64,
        total_eth_saved_wei: u128,
        total_blob_data_gas: u64,
        total_pectra_data_gas: u64,
    }

    let mut map: HashMap<String, TmpStats> = HashMap::new();

    for item in daily_txs {
        map.entry(item.batcher_address).or_default().total_daily_txs = item.tx_count;
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

    let snapshot_ts = start_ts;

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
