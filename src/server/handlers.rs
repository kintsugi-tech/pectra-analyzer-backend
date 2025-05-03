use super::types::{TxAnalysisResponse, TxHashQuery};
use crate::{
    provider::ProviderState,
    utils::{compute_calldata_gas, compute_legacy_calldata_gas},
};
use alloy_consensus::{Transaction, Typed2718};
use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::Provider;
use axum::{Json, extract::Query, extract::State};

pub async fn root_handler() -> &'static str {
    "Welcome to the pectralizer api!"
}

pub async fn tx_handler(
    State(provider_state): State<ProviderState>,
    Query(query): Query<TxHashQuery>,
) -> Json<TxAnalysisResponse> {
    // transform tx hash into a fixed bytes
    let tx_hash_bytes = FixedBytes::from_hex(&query.tx_hash).unwrap();
    // get tx
    let tx = provider_state
        .ethereum_provider
        .get_transaction_by_hash(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    // get receipt
    let receipt = provider_state
        .ethereum_provider
        .get_transaction_receipt(tx_hash_bytes)
        .await
        .unwrap()
        .unwrap();
    // get total gas used
    let gas_used = receipt.gas_used;
    let gas_price = receipt.effective_gas_price;
    if tx.is_eip4844() {
        let blob_gas_used = tx.blob_gas_used().unwrap();
        let blob_gas_price = receipt.blob_gas_price.unwrap(); // safe unwrap as it's an eip4844 tx
        let blob_data = tx.blob_versioned_hashes().unwrap();
        // get blob data
        let mut total_legacy_calldata_gas = 0;
        let mut total_eip_7623_calldata_gas = 0;
        for blob_versioned_hash in blob_data {
            let blob_data = provider_state
                .blob_provider
                .blob_data(&blob_versioned_hash.to_string())
                .await
                .unwrap();
            // compute old calldata cost with the pre eip-7623 formula
            let legacy_calldata_cost = compute_legacy_calldata_gas(&blob_data.data);
            total_legacy_calldata_gas += legacy_calldata_cost;
            // also compute new calldata cost with the eip-7623 formula
            let eip7623_calldata_cost = compute_calldata_gas(&blob_data.data);
            total_eip_7623_calldata_gas += eip7623_calldata_cost;
        }
        Json(TxAnalysisResponse {
            blob_gas_used,
            gas_used,
            gas_price,
            blob_gas_price,
            legacy_calldata_gas: total_legacy_calldata_gas,
            eip_7623_calldata_gas: total_eip_7623_calldata_gas,
        })
    } else {
        // get calldata
        let calldata = tx.input();
        // compute EIP-7623 calldata gas
        let eip_7623_calldata_gas = compute_calldata_gas(calldata);
        // compute legacy calldata gas
        let legacy_calldata_gas = compute_legacy_calldata_gas(calldata);
        Json(TxAnalysisResponse {
            blob_gas_used: 0,
            gas_used,
            gas_price,
            blob_gas_price: 0,
            eip_7623_calldata_gas,
            legacy_calldata_gas,
        })
    }
}
