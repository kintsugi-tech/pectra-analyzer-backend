use axum::{Router, routing::get};
use pectralizer::server::handlers::{root_handler, tx_handler};

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();
    // build the application
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/tx", get(tx_handler));
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
