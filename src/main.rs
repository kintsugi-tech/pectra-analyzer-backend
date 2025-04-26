use alloy_consensus::Transaction;
use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::{Provider, ProviderBuilder};
use axum::{Router, extract::Query, routing::get};
use pectralizer::{BASE_STIPEND, compute_calldata_gas};
use serde::Deserialize;
use std::env;
use url::Url;

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

async fn root_handler() -> &'static str {
    "Welcome to the pectralizer api!"
}

async fn tx_handler(Query(query): Query<TxHashQuery>) -> String {
    // ethereum infura endpoint
    let ethereum_infura_url = env::var("ETHEREUM_PROVIDER").unwrap();
    // create provider
    let provider = ProviderBuilder::new().on_http(Url::parse(&ethereum_infura_url).unwrap());
    // transform tx hash into a fixed bytes
    println!("tx_hash: {}", query.tx_hash);
    let tx_hash_bytes = FixedBytes::from_hex(&query.tx_hash).unwrap();
    // get tx
    let tx = provider
        .get_transaction_by_hash(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    // get calldata
    let calldata = tx.input();
    // get receipt
    let receipt = provider
        .get_transaction_receipt(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    // get total gas used
    let gas_used = receipt.gas_used;
    // compute EIP-7623 calldata gas
    let eip_7623_calldata_gas = compute_calldata_gas(calldata);
    // check if EIP-7623 is effective
    let is_eip_7623_effective = eip_7623_calldata_gas + BASE_STIPEND > gas_used;
    is_eip_7623_effective.to_string()
}

/// Query parameters for the tx handler
#[derive(Deserialize)]
struct TxHashQuery {
    tx_hash: String,
}
