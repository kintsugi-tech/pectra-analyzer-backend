use alloy_consensus::Transaction;
use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::{Provider, ProviderBuilder};
use std::env;
use url::Url;

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();
    // tx hash to analyze
    let tx_hash = "0xb5c6650e7faa3f6baf8dd55dc1bc1584c9a22c56fb9f80460c04a02eb65ef4c9";
    // ethereum infura endpoint
    let ethereum_infura_url = env::var("ETHEREUM_PROVIDER").unwrap();
    // create provider
    let provider = ProviderBuilder::new().on_http(Url::parse(&ethereum_infura_url).unwrap());
    // transform tx hash into a fixed bytes
    let tx_hash = FixedBytes::from_hex(tx_hash).unwrap();
    // get tx
    let tx = provider
        .get_transaction_by_hash(tx_hash)
        .await
        .unwrap()
        .unwrap();
    // get calldata
    let calldata = tx.input();
    // print calldata
    println!("calldata: {:?}", calldata);
}
