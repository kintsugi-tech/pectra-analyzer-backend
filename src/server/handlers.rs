use super::{
    error::HandlerError,
    types::{ContractAnalysisResponse, ContractQuery, TxAnalysisResponse, TxHashQuery},
};
use crate::{
    provider::ProviderState,
    utils::{compute_calldata_gas, compute_legacy_calldata_gas},
};
use alloy_consensus::{Transaction, Typed2718};
use alloy_primitives::{Address, FixedBytes, hex::FromHex};
use alloy_provider::Provider;
use axum::{Json, extract::Query, extract::State};
use reth_chainspec::NamedChain;

pub async fn root_handler() -> &'static str {
    "Welcome to the pectralizer api!"
}

async fn analyze_transaction(
    provider_state: &ProviderState,
    tx_hash_bytes: FixedBytes<32>,
) -> Result<TxAnalysisResponse, HandlerError> {
    // get tx
    let Some(tx) = provider_state
        .ethereum_provider
        .get_transaction_by_hash(tx_hash_bytes)
        .await
        .map_err(|e| {
            HandlerError::ProviderError(format!("Failed to get transaction by hash: {}", e))
        })?
    else {
        return Err(HandlerError::TransactionNotFound(tx_hash_bytes.to_string()));
    };
    // get receipt
    let Some(receipt) = provider_state
        .ethereum_provider
        .get_transaction_receipt(tx_hash_bytes)
        .await
        .map_err(|e| {
            HandlerError::ProviderError(format!("Failed to get transaction receipt: {}", e))
        })?
    else {
        return Err(HandlerError::ReceiptNotFound(tx_hash_bytes.to_string()));
    };
    // get total gas used
    let gas_used = receipt.gas_used;
    let gas_price = receipt.effective_gas_price;
    if tx.is_eip4844() {
        let blob_gas_used = tx.blob_gas_used().unwrap(); // safe unwrap as it's an eip4844 tx
        let blob_gas_price = receipt.blob_gas_price.unwrap(); // safe unwrap as it's an eip4844 tx
        let blob_data = tx.blob_versioned_hashes().unwrap(); // safe unwrap as it's an eip4844 tx
        // get blob data
        let mut total_legacy_calldata_gas = 0;
        let mut total_eip_7623_calldata_gas = 0;
        for blob_versioned_hash in blob_data {
            let blob_data = provider_state
                .blob_provider
                .get_blob_data(&blob_versioned_hash.to_string())
                .await
                .map_err(|e| {
                    HandlerError::ProviderError(format!("Failed to get blob data: {}", e))
                })?;
            // compute old calldata cost with the pre eip-7623 formula
            let legacy_calldata_cost = compute_legacy_calldata_gas(&blob_data.data);
            total_legacy_calldata_gas += legacy_calldata_cost;
            // also compute new calldata cost with the eip-7623 formula
            let eip7623_calldata_cost = compute_calldata_gas(&blob_data.data);
            total_eip_7623_calldata_gas += eip7623_calldata_cost;
        }
        Ok(TxAnalysisResponse {
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
        Ok(TxAnalysisResponse {
            blob_gas_used: 0,
            gas_used,
            gas_price,
            blob_gas_price: 0,
            eip_7623_calldata_gas,
            legacy_calldata_gas,
        })
    }
}

pub async fn tx_handler(
    State(provider_state): State<ProviderState>,
    Query(query): Query<TxHashQuery>,
) -> Result<Json<TxAnalysisResponse>, HandlerError> {
    // transform tx hash into a fixed bytes
    let tx_hash_bytes = FixedBytes::from_hex(&query.tx_hash)
        .map_err(|_| HandlerError::InvalidHex(query.tx_hash))?;
    let tx_analysis = analyze_transaction(&provider_state, tx_hash_bytes).await?;
    Ok(Json(tx_analysis))
}

pub async fn contract_handler(
    State(provider_state): State<ProviderState>,
    Query(query): Query<ContractQuery>,
) -> Result<Json<ContractAnalysisResponse>, HandlerError> {
    let contract_address = Address::from_hex(&query.contract_address)
        .map_err(|_| HandlerError::InvalidHex(query.contract_address))?;
    let last_block_number = provider_state
        .ethereum_provider
        .get_block_number()
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get block number: {}", e)))?;
    // 700k blocks are roughly 3 months in Ethereum mainnet with 12s block time
    let start_block = last_block_number - 700_000;
    let chain_id = NamedChain::Mainnet.into();
    // collect all transaction hashes into a single Vec directly
    let mut tx_hashes = Vec::new();
    // get internal transactions
    let internal_txs = provider_state
        .etherscan_provider
        .get_internal_txs(contract_address, chain_id, start_block, last_block_number)
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get internal txs: {}", e)))?;
    tx_hashes.extend(internal_txs.result.iter().map(|tx| tx.hash));
    // get normal transactions
    let normal_txs = provider_state
        .etherscan_provider
        .get_normal_txs(contract_address, chain_id, start_block, last_block_number)
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get normal txs: {}", e)))?;
    tx_hashes.extend(normal_txs.result.iter().map(|tx| tx.hash));
    let mut txs_analysis = vec![];
    for tx_hash in tx_hashes {
        let tx_analysis = analyze_transaction(&provider_state, tx_hash).await?;
        txs_analysis.push(tx_analysis);
    }
    Ok(Json(ContractAnalysisResponse { txs_analysis }))
}
