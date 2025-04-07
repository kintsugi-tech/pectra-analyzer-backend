use alloy_consensus::Transaction;
use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::{Provider, ProviderBuilder};
use pectralizer::{BASE_STIPEND, compute_calldata_gas};
use std::env;
use url::Url;

#[tokio::main]
async fn main() {
    // load .env environment variables
    dotenv::dotenv().ok();
    // tx hash to analyze
    let tx_hash = "0x17ee587040c06bf85ee426eb975c737129d1082f2aaacc61abfedd0a9deb69a9";
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
    // get receipt
    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();
    // get total gas used
    let gas_used = receipt.gas_used;
    println!("Gas used: {}", gas_used);
    // compute EIP-7623 calldata gas
    let eip_7623_calldata_gas = compute_calldata_gas(calldata);
    println!("EIP-7623 calldata gas: {}", eip_7623_calldata_gas);
    // check if EIP-7623 is effective
    let is_eip_7623_effective = eip_7623_calldata_gas + BASE_STIPEND > gas_used;
    println!("EIP-7623 is effective: {}", is_eip_7623_effective);
}
