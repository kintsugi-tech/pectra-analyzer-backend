use axum::{Router, routing::get};
use pectralizer::server::handlers::{root_handler, tx_handler};

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();

    // Validate required environment variables
    if std::env::var("ETHEREUM_PROVIDER").is_err() {
        eprintln!("Error: ETHEREUM_PROVIDER environment variable is not set");
        std::process::exit(1);
    }

    // Get port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port.parse().expect("PORT must be a valid number");

    // build the application
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/tx", get(tx_handler));

    // run our app with hyper, listening globally on configured port
    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 Starting Pectralizer server...");
    println!("📡 Ethereum provider configured");
    println!("🌐 Server listening on http://0.0.0.0:{}", port);
    println!("📝 Available endpoints:");
    println!("   - GET  /     - Welcome message");
    println!("   - GET  /tx   - Transaction analysis");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use pectralizer::server::{handlers::tx_handler, types::TxHashQuery};

    #[tokio::test]
    async fn test_tx_handler() {
        let query = TxHashQuery {
            tx_hash: "0xf9b3708d3c8a07f7c26bbd336c2746977787b126fbc95e2df816a74d599957c4"
                .to_string(),
        };
        let response = tx_handler(Query(query)).await;
        println!("response: {:?}", response.0);
    }
}
