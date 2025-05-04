use axum::{Router, routing::get};
use pectralizer::{
    provider::ProviderState,
    server::handlers::{contract_handler, root_handler, tx_handler},
};

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();

    // Validate required environment variables
    let ethereum_provider_url = std::env::var("ETHEREUM_PROVIDER")
        .expect("ETHEREUM_PROVIDER environment variable is not set");
    let etherscan_api_key = std::env::var("ETHERSCAN_API_KEY")
        .expect("ETHERSCAN_API_KEY environment variable is not set");

    // Initialize shared provider state
    let provider_state = ProviderState::new(&ethereum_provider_url, &etherscan_api_key).await;

    // Get port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port.parse().expect("PORT must be a valid number");

    // build the application
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/tx", get(tx_handler))
        .route("/contract", get(contract_handler))
        .with_state(provider_state);

    // run our app with hyper, listening globally on configured port
    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 Starting Pectralizer server...");
    println!("📡 Ethereum provider configured");
    println!("🌐 Server listening on http://0.0.0.0:{}", port);
    println!("📝 Available endpoints:");
    println!("   - GET  /           - Welcome message");
    println!("   - GET  /tx         - Transaction analysis");
    println!("   - GET  /contract   - Contract analysis");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use axum::extract::{Query, State};
    use pectralizer::{
        provider::ProviderState,
        server::{
            handlers::tx_handler,
            types::{TxAnalysisResponse, TxHashQuery},
        },
    };

    #[tokio::test]
    async fn test_eip1559_tx() {
        let provider_state =
            ProviderState::new("https://eth.merkle.io", "https://eth.merkle.io").await;
        let query = TxHashQuery {
            tx_hash: "0xd367c556c43058a3718362a0b2e624471c69e7f00846fe4474469a9895310bbd"
                .to_string(),
        };
        let response = tx_handler(State(provider_state), Query(query)).await;
        let expected_response = TxAnalysisResponse {
            gas_used: 74557,
            gas_price: 1014646161,
            blob_gas_price: 0,
            blob_gas_used: 0,
            eip_7623_calldata_gas: 13430,
            legacy_calldata_gas: 5372,
        };
        assert_eq!(response.0, expected_response);
    }

    #[tokio::test]
    async fn test_blob_tx() {
        let provider_state =
            ProviderState::new("https://eth.merkle.io", "https://eth.merkle.io").await;
        let query = TxHashQuery {
            tx_hash: "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
                .to_string(),
        };
        let response = tx_handler(State(provider_state), Query(query)).await;
        let expected_response = TxAnalysisResponse {
            gas_used: 21000,
            gas_price: 5767832048,
            blob_gas_price: 2793617096,
            blob_gas_used: 393216,
            eip_7623_calldata_gas: 15574830,
            legacy_calldata_gas: 6229932,
        };
        assert_eq!(response.0, expected_response);
    }
}
