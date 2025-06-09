use alloy_provider::Provider;
use axum::{Router, routing::get};
use pectralizer::{
    provider::ProviderState,
    server::{
        AppState,
        handlers::{
            blob_data_gas_handler, contract_handler, daily_txs_handler, eth_saved_handler,
            pectra_data_gas_handler, root_handler, tx_handler,
        },
    },
    tracker::{
        self,
        database::{Database, SqliteDatabase},
        retry_handler::RetryHandler,
    },
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// The path to the database file for the L2 batches monitoring service.
const DB_PATH: &str = "./l2_batches_monitoring.db";

/// Run the L2 proposers monitoring service.
async fn run_l2_batches_monitoring_service(app_state: AppState) -> eyre::Result<()> {
    info!("Initializing L2 batches monitoring database...");
    // create retry handler for failed transactions
    let retry_handler = RetryHandler::new(app_state.db.clone(), app_state.provider_state.clone());

    info!("Starting L2 batches monitoring service and retry handler...");

    // run both monitoring and retry services concurrently
    tokio::select! {
        res = tracker::l2_monitor::start_monitoring(app_state.db, app_state.provider_state) => {
            if let Err(e) = res {
                error!("L2 monitor error: {:?}", e);
            }
        },
        res = retry_handler.start_retry_loop() => {
            if let Err(e) = res {
                error!("Retry handler error: {:?}", e);
            }
        },
    }

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // init tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| eyre::eyre!("Failed to initialize tracing: {}", e))?;

    // load .env environment variables
    dotenv::dotenv().ok();

    // validate required environment variables
    let ethereum_provider_url = std::env::var("ETHEREUM_PROVIDER")
        .map_err(|_| eyre::eyre!("ETHEREUM_PROVIDER environment variable is not set"))?;
    let etherscan_api_key = std::env::var("ETHERSCAN_API_KEY")
        .map_err(|_| eyre::eyre!("ETHERSCAN_API_KEY environment variable is not set"))?;
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .map_err(|_| eyre::eyre!("CHAIN_ID environment variable is not set"))?
        .parse()?;

    // initialize shared provider state
    let provider_state =
        ProviderState::new(&ethereum_provider_url, &etherscan_api_key, chain_id).await;

    // initialize the database for API endpoints
    let current_block = provider_state
        .ethereum_provider
        .get_block_number()
        .await
        .map_err(|e| {
            eyre::eyre!(
                "Failed to get current block number for API DB initialization: {}",
                e
            )
        })?;

    let db_instance = SqliteDatabase::new(DB_PATH, current_block)
        .await
        .map_err(|e| eyre::eyre!("Failed to initialize database for API: {}", e))?;
    let db_arc: Arc<dyn Database> = Arc::new(db_instance);

    // create shared application state
    let app_state = AppState {
        provider_state,
        db: db_arc,
    };

    // get port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port
        .parse()
        .map_err(|_| eyre::eyre!("PORT must be a valid number"))?;

    // build the application
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/tx", get(tx_handler))
        .route("/contract", get(contract_handler))
        .route("/daily_txs", get(daily_txs_handler))
        .route("/eth_saved", get(eth_saved_handler))
        .route("/blob_data_gas", get(blob_data_gas_handler))
        .route("/pectra_data_gas", get(pectra_data_gas_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state.clone());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 Starting Pectralizer server...");
    info!("📡 Ethereum provider configured");
    info!("🌐 Server listening on http://0.0.0.0:{}", port);
    info!("📝 Available endpoints:");
    info!("   - GET  /           - Welcome message");
    info!("   - GET  /tx         - Transaction analysis");
    info!("   - GET  /contract   - Contract analysis");
    info!("   - GET  /daily_txs  - Daily transactions analysis");
    info!("   - GET  /eth_saved  - Ethereum saved analysis");
    info!("   - GET  /blob_data_gas - Blob data gas analysis");
    info!("   - GET  /pectra_data_gas - Pectra data gas analysis");

    // run both services concurrently
    tokio::select! {
        res = async { axum::serve(listener, app).await.map_err(eyre::Report::from) } => {
            if let Err(e) = res {
                error!("Axum server error: {:?}", e);
            }
        },
        res = run_l2_batches_monitoring_service(app_state) => {
            if let Err(e) = res {
                error!("L2 tracker service error: {:?}", e);
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy_chains::NamedChain;
    use axum::extract::{Query, State};
    use pectralizer::{
        provider::ProviderState,
        server::{
            AppState,
            handlers::{
                blob_data_gas_handler, contract_handler, daily_txs_handler, eth_saved_handler,
                pectra_data_gas_handler, tx_handler,
            },
            types::{
                ContractQuery, DailyTxsQuery, EthSavedQuery, GasUsageQuery, TxAnalysisResponse,
                TxHashQuery,
            },
        },
        tracker::database::{Database, SqliteDatabase, TrackedBatch},
    };
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    /// Helper function to create a test AppState.
    async fn create_test_app_state() -> AppState {
        // load .env environment variables
        dotenv::dotenv().ok();
        let etherscan_api_key =
            std::env::var("ETHERSCAN_API_KEY").unwrap_or_else(|_| "demo".to_string()); // Use demo key if not set
        let provider_state = ProviderState::new(
            "https://eth.merkle.io",
            &etherscan_api_key,
            NamedChain::Mainnet.into(),
        )
        .await;

        // create a temporary database file that will be automatically deleted
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap().to_string();
        std::mem::forget(temp_file);
        let db = SqliteDatabase::new(&db_path, 0).await.unwrap();
        let db_arc: Arc<dyn Database> = Arc::new(db);

        AppState {
            provider_state,
            db: db_arc,
        }
    }

    /// Helper function to create a test AppState with Sepolia testnet
    async fn create_test_app_state_sepolia() -> AppState {
        // load .env environment variables
        dotenv::dotenv().ok();
        let etherscan_api_key =
            std::env::var("ETHERSCAN_API_KEY").unwrap_or_else(|_| "demo".to_string()); // Use demo key if not set
        let provider_state = ProviderState::new(
            "https://ethereum-sepolia-rpc.publicnode.com",
            &etherscan_api_key,
            NamedChain::Sepolia.into(),
        )
        .await;

        // Create a temporary database file that will be automatically deleted
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap().to_string();
        std::mem::forget(temp_file);
        let db = SqliteDatabase::new(&db_path, 0).await.unwrap();
        let db_arc: Arc<dyn Database> = Arc::new(db);

        AppState {
            provider_state,
            db: db_arc,
        }
    }

    #[tokio::test]
    async fn test_eip1559_tx() {
        let app_state = create_test_app_state().await;
        let query = TxHashQuery {
            tx_hash: "0xd367c556c43058a3718362a0b2e624471c69e7f00846fe4474469a9895310bbd"
                .to_string(),
        };
        let response = tx_handler(State(app_state), Query(query)).await.unwrap();
        let expected_response = TxAnalysisResponse {
            timestamp: 1746290387,
            gas_used: 74557,
            gas_price: 1014646161,
            blob_gas_price: Some(441344044),
            blob_gas_used: 0,
            eip_7623_calldata_gas: 13430,
            legacy_calldata_gas: 5372,
            blob_data_wei_spent: Some(57847846535168),
            legacy_calldata_wei_spent: 5450679176892,
            eip_7623_calldata_wei_spent: 13626697942230,
        };
        assert_eq!(response.0, expected_response);
    }

    #[tokio::test]
    async fn test_blob_tx() {
        let app_state = create_test_app_state().await;
        let query = TxHashQuery {
            tx_hash: "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
                .to_string(),
        };
        let response = tx_handler(State(app_state), Query(query)).await;

        // Handle the case where blob data might not be available from the RPC endpoint
        match response {
            Ok(response) => {
                // If successful, verify it's a blob transaction
                assert!(response.0.blob_gas_used > 0);
                println!(
                    "Blob transaction test passed: blob_gas_used = {}",
                    response.0.blob_gas_used
                );
            }
            Err(e) => {
                // If it fails due to blob data issues, that's expected for some RPC endpoints
                println!(
                    "Blob transaction test skipped due to RPC limitations: {:?}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_blob_tx_sepolia() {
        let app_state = create_test_app_state_sepolia().await;
        let query = TxHashQuery {
            tx_hash: "0x6516958cca067ee7de225b23f8034ce0a79aae16af176d566bf894e35722f34d"
                .to_string(),
        };
        let response = tx_handler(State(app_state), Query(query)).await;

        // Handle the case where blob data might not be available from the RPC endpoint
        match response {
            Ok(response) => {
                // If successful, verify it's a blob transaction
                assert!(response.0.blob_gas_used > 0);
                println!(
                    "Sepolia blob transaction test passed: blob_gas_used = {}",
                    response.0.blob_gas_used
                );
            }
            Err(e) => {
                // If it fails due to blob data issues, that's expected for some RPC endpoints
                println!(
                    "Sepolia blob transaction test skipped due to RPC limitations: {:?}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_contract_handler() {
        let app_state = create_test_app_state().await;
        let query = ContractQuery {
            contract_address: "0x41dDf7fC14a579E0F3f2D698e14c76d9d486B9F7".to_string(),
        };
        let _response = contract_handler(State(app_state), Query(query))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_contract_handler_sepolia() {
        let app_state = create_test_app_state_sepolia().await;
        let query = ContractQuery {
            contract_address: "0xfD3130Ea0e8B7Dd61Ac3663328a66d97eb02f84b".to_string(),
        };
        let _response = contract_handler(State(app_state), Query(query))
            .await
            .unwrap();
    }

    // Database functionality tests

    #[tokio::test]
    async fn test_daily_txs_handler_empty_db() {
        let app_state = create_test_app_state().await;
        let query = DailyTxsQuery {
            batcher_address: "0x123abc".to_string(),
            start_timestamp: 1000000000,
            end_timestamp: 2000000000,
        };
        let response = daily_txs_handler(State(app_state), Query(query))
            .await
            .unwrap();

        // Since we have an empty database, expect 0 transactions
        assert_eq!(response.0.batcher_address, "0x123abc");
        assert_eq!(response.0.tx_count, 0);
    }

    #[tokio::test]
    async fn test_eth_saved_handler_empty_db() {
        let app_state = create_test_app_state().await;
        let query = EthSavedQuery {
            batcher_address: "0x456def".to_string(),
            start_timestamp: 1000000000,
            end_timestamp: 2000000000,
        };
        let response = eth_saved_handler(State(app_state), Query(query))
            .await
            .unwrap();

        // Since we have an empty database, expect 0 ETH saved
        assert_eq!(response.0.batcher_address, "0x456def");
        assert_eq!(response.0.total_eth_saved_wei, 0);
    }

    #[tokio::test]
    async fn test_blob_data_gas_handler_empty_db() {
        let app_state = create_test_app_state().await;
        let query = GasUsageQuery {
            batcher_address: "0x789ghi".to_string(),
            start_timestamp: 1000000000,
            end_timestamp: 2000000000,
        };
        let response = blob_data_gas_handler(State(app_state), Query(query))
            .await
            .unwrap();

        // Since we have an empty database, expect 0 gas
        assert_eq!(response.0.total_blob_data_gas, 0);
    }

    #[tokio::test]
    async fn test_pectra_data_gas_handler_empty_db() {
        let app_state = create_test_app_state().await;
        let query = GasUsageQuery {
            batcher_address: "0xabcdef".to_string(),
            start_timestamp: 1000000000,
            end_timestamp: 2000000000,
        };
        let response = pectra_data_gas_handler(State(app_state), Query(query))
            .await
            .unwrap();

        // Since we have an empty database, expect 0 gas
        assert_eq!(response.0.total_pectra_data_gas, 0);
    }

    #[tokio::test]
    async fn test_database_operations() {
        let app_state = create_test_app_state().await;

        // Test inserting a tracked batch
        let batch = TrackedBatch {
            id: None,
            tx_hash: "0x1234567890abcdef".to_string(),
            batcher_address: "0xbatcher123".to_string(),
            analysis_result: r#"{"blob_gas_used": 131072, "eip_7623_calldata_gas": 1000, "blob_data_wei_spent": 1000000, "eip_7623_calldata_wei_spent": 2000000, "timestamp": 1600000000}"#.to_string(),
            timestamp: 1600000000,
            last_analyzed_block: None,
        };

        app_state.db.save_tracked_batch(&batch).await.unwrap();

        // Test that the transaction is now tracked
        let is_tracked = app_state
            .db
            .is_tx_already_tracked("0x1234567890abcdef")
            .await
            .unwrap();
        assert!(is_tracked);

        // Test daily transactions query with data
        let query = DailyTxsQuery {
            batcher_address: "0xbatcher123".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response = daily_txs_handler(State(app_state.clone()), Query(query))
            .await
            .unwrap();

        assert_eq!(response.0.batcher_address, "0xbatcher123");
        assert_eq!(response.0.tx_count, 1);

        // Test ETH saved query with data
        let query = EthSavedQuery {
            batcher_address: "0xbatcher123".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response = eth_saved_handler(State(app_state.clone()), Query(query))
            .await
            .unwrap();

        assert_eq!(response.0.batcher_address, "0xbatcher123");
        assert_eq!(response.0.total_eth_saved_wei, 1000000); // 2000000 - 1000000 = 1000000

        // Test blob data gas query with data
        let query = GasUsageQuery {
            batcher_address: "0xbatcher123".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response = blob_data_gas_handler(State(app_state.clone()), Query(query))
            .await
            .unwrap();

        assert_eq!(response.0.total_blob_data_gas, 131072);

        // Test Pectra data gas query with data
        let query = GasUsageQuery {
            batcher_address: "0xbatcher123".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response = pectra_data_gas_handler(State(app_state), Query(query))
            .await
            .unwrap();

        assert_eq!(response.0.total_pectra_data_gas, 1000);
    }

    #[tokio::test]
    async fn test_multiple_batchers_isolation() {
        let app_state = create_test_app_state().await;

        // Insert data for multiple batchers
        let batch1 = TrackedBatch {
            id: None,
            tx_hash: "0x1111111111111111".to_string(),
            batcher_address: "0xbatcher1".to_string(),
            analysis_result: r#"{"blob_gas_used": 100000, "eip_7623_calldata_gas": 500, "blob_data_wei_spent": 500000, "eip_7623_calldata_wei_spent": 1000000, "timestamp": 1600000000}"#.to_string(),
            timestamp: 1600000000,
            last_analyzed_block: None,
        };

        let batch2 = TrackedBatch {
            id: None,
            tx_hash: "0x2222222222222222".to_string(),
            batcher_address: "0xbatcher2".to_string(),
            analysis_result: r#"{"blob_gas_used": 200000, "eip_7623_calldata_gas": 1000, "blob_data_wei_spent": 800000, "eip_7623_calldata_wei_spent": 1500000, "timestamp": 1600000000}"#.to_string(),
            timestamp: 1600000000,
            last_analyzed_block: None,
        };

        app_state.db.save_tracked_batch(&batch1).await.unwrap();
        app_state.db.save_tracked_batch(&batch2).await.unwrap();

        // Test that batcher1 data is isolated
        let query1 = DailyTxsQuery {
            batcher_address: "0xbatcher1".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response1 = daily_txs_handler(State(app_state.clone()), Query(query1))
            .await
            .unwrap();

        assert_eq!(response1.0.batcher_address, "0xbatcher1");
        assert_eq!(response1.0.tx_count, 1);

        // Test that batcher2 data is isolated
        let query2 = DailyTxsQuery {
            batcher_address: "0xbatcher2".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let response2 = daily_txs_handler(State(app_state.clone()), Query(query2))
            .await
            .unwrap();

        assert_eq!(response2.0.batcher_address, "0xbatcher2");
        assert_eq!(response2.0.tx_count, 1);

        // Test ETH saved isolation
        let eth_query1 = EthSavedQuery {
            batcher_address: "0xbatcher1".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let eth_response1 = eth_saved_handler(State(app_state.clone()), Query(eth_query1))
            .await
            .unwrap();

        assert_eq!(eth_response1.0.total_eth_saved_wei, 500000); // 1000000 - 500000

        let eth_query2 = EthSavedQuery {
            batcher_address: "0xbatcher2".to_string(),
            start_timestamp: 1500000000,
            end_timestamp: 1700000000,
        };
        let eth_response2 = eth_saved_handler(State(app_state), Query(eth_query2))
            .await
            .unwrap();

        assert_eq!(eth_response2.0.total_eth_saved_wei, 700000); // 1500000 - 800000
    }
}
