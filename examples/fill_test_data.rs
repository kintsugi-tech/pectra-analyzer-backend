use pectralizer::server::types::TxAnalysisResponse;
use pectralizer::tracker::database::{Database, SqliteDatabase, TrackedBatch};
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

    // Use exact addresses from L2_BATCHERS_ADDRESSES in l2_monitor.rs
    let base_batcher = "0x5050f69a9786f081509234f1a7f4684b5e5b76c9"; // Base - lowercase hex format
    let optimism_batcher = "0x6887246668a3b87f54deb3b94ba47a6f63f32985"; // Optimism - lowercase hex format

    // Create proper TxAnalysisResponse structures for test data
    let create_analysis_response = |timestamp: u64,
                                    gas_used: u64,
                                    gas_price: u128,
                                    blob_gas_price: u128,
                                    blob_gas_used: u64,
                                    eip_7623_calldata_gas: u64,
                                    legacy_calldata_gas: u64|
     -> String {
        let response = TxAnalysisResponse {
            timestamp,
            gas_used,
            gas_price,
            blob_gas_price: Some(blob_gas_price),
            blob_gas_used,
            eip_7623_calldata_gas,
            legacy_calldata_gas,
            blob_data_wei_spent: Some(blob_gas_used as u128 * blob_gas_price),
            legacy_calldata_wei_spent: legacy_calldata_gas as u128 * gas_price,
            eip_7623_calldata_wei_spent: eip_7623_calldata_gas as u128 * gas_price,
        };
        serde_json::to_string(&response).unwrap()
    };

    // Base batcher test data
    let base_batches = vec![
        TrackedBatch {
            id: None,
            tx_hash: "0xbase1111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            batcher_address: base_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 86400) as u64, // 1 day ago
                150000,               // gas_used
                20_000_000_000,       // gas_price (20 gwei)
                15_000_000_000,       // blob_gas_price (15 gwei)
                131072,               // blob_gas_used
                15000,                // eip_7623_calldata_gas
                12000,                // legacy_calldata_gas
            ),
            timestamp: now - 86400, // 1 day ago
            last_analyzed_block: None,
        },
        TrackedBatch {
            id: None,
            tx_hash: "0xbase2222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            batcher_address: base_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 82800) as u64, // 23 hours ago
                200000,               // gas_used
                25_000_000_000,       // gas_price (25 gwei)
                18_000_000_000,       // blob_gas_price (18 gwei)
                262144,               // blob_gas_used
                25000,                // eip_7623_calldata_gas
                20000,                // legacy_calldata_gas
            ),
            timestamp: now - 82800, // 23 hours ago
            last_analyzed_block: None,
        },
        TrackedBatch {
            id: None,
            tx_hash: "0xbase3333333333333333333333333333333333333333333333333333333333333"
                .to_string(),
            batcher_address: base_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 79200) as u64, // 22 hours ago
                175000,               // gas_used
                22_000_000_000,       // gas_price (22 gwei)
                16_000_000_000,       // blob_gas_price (16 gwei)
                196608,               // blob_gas_used
                18000,                // eip_7623_calldata_gas
                15000,                // legacy_calldata_gas
            ),
            timestamp: now - 79200, // 22 hours ago
            last_analyzed_block: None,
        },
    ];

    // Optimism batcher test data
    let optimism_batches = vec![
        TrackedBatch {
            id: None,
            tx_hash: "0xop111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            batcher_address: optimism_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 75600) as u64, // 21 hours ago
                300000,               // gas_used
                30_000_000_000,       // gas_price (30 gwei)
                20_000_000_000,       // blob_gas_price (20 gwei)
                393216,               // blob_gas_used
                35000,                // eip_7623_calldata_gas
                28000,                // legacy_calldata_gas
            ),
            timestamp: now - 75600, // 21 hours ago
            last_analyzed_block: None,
        },
        TrackedBatch {
            id: None,
            tx_hash: "0xop222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            batcher_address: optimism_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 72000) as u64, // 20 hours ago
                350000,               // gas_used
                35_000_000_000,       // gas_price (35 gwei)
                25_000_000_000,       // blob_gas_price (25 gwei)
                524288,               // blob_gas_used
                45000,                // eip_7623_calldata_gas
                36000,                // legacy_calldata_gas
            ),
            timestamp: now - 72000, // 20 hours ago
            last_analyzed_block: None,
        },
        TrackedBatch {
            id: None,
            tx_hash: "0xop333333333333333333333333333333333333333333333333333333333333333"
                .to_string(),
            batcher_address: optimism_batcher.to_string(),
            analysis_result: create_analysis_response(
                (now - 68400) as u64, // 19 hours ago
                250000,               // gas_used
                28_000_000_000,       // gas_price (28 gwei)
                22_000_000_000,       // blob_gas_price (22 gwei)
                327680,               // blob_gas_used
                28000,                // eip_7623_calldata_gas
                22000,                // legacy_calldata_gas
            ),
            timestamp: now - 68400, // 19 hours ago
            last_analyzed_block: None,
        },
    ];

    // Insert Base batches
    println!("📊 Inserting Base batcher data...");
    for batch in base_batches {
        match db.save_tracked_batch(&batch).await {
            Ok(_) => println!("  ✅ Inserted batch: {}", batch.tx_hash),
            Err(e) => println!("  ❌ Failed to insert batch {}: {}", batch.tx_hash, e),
        }
    }

    // Insert Optimism batches
    println!("📊 Inserting Optimism batcher data...");
    for batch in optimism_batches {
        match db.save_tracked_batch(&batch).await {
            Ok(_) => println!("  ✅ Inserted batch: {}", batch.tx_hash),
            Err(e) => println!("  ❌ Failed to insert batch {}: {}", batch.tx_hash, e),
        }
    }

    // Add some older data for testing different time ranges
    println!("📊 Inserting older test data...");
    let older_batch = TrackedBatch {
        id: None,
        tx_hash: "0xold1111111111111111111111111111111111111111111111111111111111111".to_string(),
        batcher_address: base_batcher.to_string(),
        analysis_result: create_analysis_response(
            (now - 604800) as u64, // 7 days ago
            120000,                // gas_used
            15_000_000_000,        // gas_price (15 gwei)
            12_000_000_000,        // blob_gas_price (12 gwei)
            131072,                // blob_gas_used
            12000,                 // eip_7623_calldata_gas
            10000,                 // legacy_calldata_gas
        ),
        timestamp: now - 604800, // 7 days ago
        last_analyzed_block: None,
    };

    match db.save_tracked_batch(&older_batch).await {
        Ok(_) => println!("  ✅ Inserted older batch: {}", older_batch.tx_hash),
        Err(e) => println!("  ❌ Failed to insert older batch: {}", e),
    }

    println!("\n🎉 Test data insertion completed!");
    println!("📈 Summary:");
    println!("  - 3 Base batcher transactions");
    println!("  - 3 Optimism batcher transactions");
    println!("  - 1 older transaction for time range testing");
    println!("\n🔍 You can now test the API endpoints with:");
    println!("  Base batcher: {}", base_batcher);
    println!("  Optimism batcher: {}", optimism_batcher);
    println!("  Time range: {} to {}", now - 86400, now);

    Ok(())
}
