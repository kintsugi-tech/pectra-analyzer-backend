use axum::{Router, routing::get};
use pectralizer::{
    provider::ProviderState,
    server::handlers::{contract_handler, root_handler, tx_handler},
    tracker,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use alloy_provider::Provider;

/// The path to the database file for the L2 batches monitoring service.
const DB_PATH: &str = "./l2_batches_monitoring.db";

/// Run the L2 proposers monitoring service.
async fn run_l2_batches_monitoring_service(provider_state: ProviderState) -> eyre::Result<()> {
    println!("Initializing L2 batches monitoring database...");

    // get current block number for initial setup
    let initial_block = provider_state.ethereum_provider.get_block_number().await
        .map_err(|e| eyre::eyre!("Failed to get current block number for DB initialization: {}", e))?;

    // initialize the database
    let db_conn = tracker::db::initialize_db(DB_PATH, initial_block)
        .map_err(|e| eyre::eyre!("Failed to initialize L2 batches monitoring database: {}", e))?;
    let db_conn_arc = Arc::new(db_conn);

    println!("Starting L2 batches monitoring service...");
    tracker::l2_monitor::start_monitoring(db_conn_arc, provider_state).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
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
    let provider_state_for_tracker = provider_state.clone();

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
        .layer(CorsLayer::permissive())
        .with_state(provider_state.clone());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("🚀 Starting Pectralizer server...");
    println!("📡 Ethereum provider configured");
    println!("🌐 Server listening on http://0.0.0.0:{}", port);
    println!("📝 Available endpoints:");
    println!("   - GET  /           - Welcome message");
    println!("   - GET  /tx         - Transaction analysis");
    println!("   - GET  /contract   - Contract analysis");

    // run both services concurrently
    tokio::select! {
        res = async { axum::serve(listener, app).await.map_err(eyre::Report::from) } => {
            if let Err(e) = res {
                eprintln!("Axum server error: {:?}", e);
            }
        },
        res = run_l2_batches_monitoring_service(provider_state_for_tracker) => {
            if let Err(e) = res {
                eprintln!("L2 tracker service error: {:?}", e);
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
            handlers::{contract_handler, tx_handler},
            types::{ContractQuery, TxAnalysisResponse, TxHashQuery},
        },
    };

    #[tokio::test]
    async fn test_eip1559_tx() {
        let provider_state = ProviderState::new(
            "https://eth.merkle.io",
            "https://eth.merkle.io",
            NamedChain::Mainnet.into(),
        )
        .await;
        let query = TxHashQuery {
            tx_hash: "0xd367c556c43058a3718362a0b2e624471c69e7f00846fe4474469a9895310bbd"
                .to_string(),
        };
        let response = tx_handler(State(provider_state), Query(query))
            .await
            .unwrap();
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
        let provider_state = ProviderState::new(
            "https://eth.merkle.io",
            "https://eth.merkle.io",
            NamedChain::Mainnet.into(),
        )
        .await;
        let query = TxHashQuery {
            tx_hash: "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
                .to_string(),
        };
        let response = tx_handler(State(provider_state), Query(query))
            .await
            .unwrap();
        let expected_response = TxAnalysisResponse {
            timestamp: 1745681771,
            gas_used: 21000,
            gas_price: 5767832048,
            blob_gas_price: Some(2793617096),
            blob_gas_used: 393216,
            eip_7623_calldata_gas: 15574830,
            legacy_calldata_gas: 6229932,
            blob_data_wei_spent: Some(1098494940020736),
            legacy_calldata_wei_spent: 35933201446460736,
            eip_7623_calldata_wei_spent: 89833003616151840,
        };
        assert_eq!(response.0, expected_response);
    }

    #[tokio::test]
    async fn test_blob_tx_sepolia() {
        let provider_state = ProviderState::new(
            "https://ethereum-sepolia-rpc.publicnode.com",
            "https://ethereum-sepolia-rpc.publicnode.com",
            NamedChain::Sepolia.into(),
        )
        .await;
        let query = TxHashQuery {
            tx_hash: "0x6516958cca067ee7de225b23f8034ce0a79aae16af176d566bf894e35722f34d"
                .to_string(),
        };
        let response = tx_handler(State(provider_state), Query(query))
            .await
            .unwrap();
        let expected_response = TxAnalysisResponse {
            timestamp: 1747858644,
            gas_used: 21000,
            gas_price: 1000000038,
            blob_gas_price: Some(1),
            blob_gas_used: 131072,
            eip_7623_calldata_gas: 1339520,
            legacy_calldata_gas: 535808,
            blob_data_wei_spent: Some(131072),
            legacy_calldata_wei_spent: 535808020360704,
            eip_7623_calldata_wei_spent: 1339520050901760,
        };
        assert_eq!(response.0, expected_response);
    }

    #[tokio::test]
    async fn test_contract_handler() {
        // load .env environment variables
        dotenv::dotenv().ok();
        let etherscan_api_key = std::env::var("ETHERSCAN_API_KEY")
            .map_err(|_| eyre::eyre!("ETHERSCAN_API_KEY environment variable is not set"))
            .unwrap();
        let provider_state = ProviderState::new(
            "https://eth.merkle.io",
            &etherscan_api_key,
            NamedChain::Mainnet.into(),
        )
        .await;
        let query = ContractQuery {
            contract_address: "0x41dDf7fC14a579E0F3f2D698e14c76d9d486B9F7".to_string(),
        };
        let _response = contract_handler(State(provider_state), Query(query))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_contract_handler_sepolia() {
        // load .env environment variables
        dotenv::dotenv().ok();
        let etherscan_api_key = std::env::var("ETHERSCAN_API_KEY")
            .map_err(|_| eyre::eyre!("ETHERSCAN_API_KEY environment variable is not set"))
            .unwrap();
        let provider_state = ProviderState::new(
            "https://ethereum-sepolia-rpc.publicnode.com",
            &etherscan_api_key,
            NamedChain::Sepolia.into(),
        )
        .await;
        let query = ContractQuery {
            contract_address: "0xfD3130Ea0e8B7Dd61Ac3663328a66d97eb02f84b".to_string(),
        };
        let _response = contract_handler(State(provider_state), Query(query))
            .await
            .unwrap();
    }
}
