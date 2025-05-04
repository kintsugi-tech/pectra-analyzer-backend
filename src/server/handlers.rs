use super::types::{ContractAnalysisResponse, ContractQuery, TxAnalysisResponse, TxHashQuery};
use crate::{
    provider::ProviderState,
    utils::{compute_calldata_gas, compute_legacy_calldata_gas},
};
use alloy_consensus::{Transaction, Typed2718};
use alloy_primitives::{Address, FixedBytes, TxHash, hex::FromHex};
use alloy_provider::Provider;
use axum::{Json, extract::Query, extract::State};
use reth_chainspec::NamedChain;

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

pub async fn contract_handler(
    State(provider_state): State<ProviderState>,
    Query(query): Query<ContractQuery>,
) -> Json<ContractAnalysisResponse> {
    let contract_address = Address::from_hex(&query.contract_address).unwrap();
    let last_block_number = provider_state
        .ethereum_provider
        .get_block_number()
        .await
        .unwrap();
    // 700k blocks are roughly 3 months in Ethereum mainnet with 12s block time
    let start_block = last_block_number - 700_000;
    let chain_id = NamedChain::Mainnet.into();
    let internal_txs = provider_state
        .etherscan_provider
        .get_internal_txs(contract_address, chain_id, start_block, last_block_number)
        .await
        .unwrap();
    let internal_txs: Vec<TxHash> = internal_txs.result.iter().map(|tx| tx.hash).collect();
    let normal_txs = provider_state
        .etherscan_provider
        .get_normal_txs(contract_address, chain_id, start_block, last_block_number)
        .await
        .unwrap();
    let normal_txs: Vec<TxHash> = normal_txs.result.iter().map(|tx| tx.hash).collect();
    // concatenate internal and normal tx hashes
    let tx_hashes = [internal_txs, normal_txs].concat();
    let mut txs_analysis = vec![];
    for tx_hash in tx_hashes {
        let tx_query = TxHashQuery {
            tx_hash: tx_hash.to_string(),
        };
        let tx_response = tx_handler(State(provider_state.clone()), Query(tx_query))
            .await
            .0;
        txs_analysis.push(tx_response);
    }
    Json(ContractAnalysisResponse { txs_analysis })
}
