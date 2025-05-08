use super::{
    error::HandlerError,
    types::{ContractAnalysisResponse, ContractQuery, TxAnalysisResponse, TxHashQuery},
};
use crate::{
    provider::ProviderState,
    utils::{BASE_STIPEND, BYTES_PER_BLOB, compute_calldata_gas, compute_legacy_calldata_gas},
};
use alloy_consensus::{Transaction, Typed2718};
use alloy_primitives::{Address, FixedBytes, hex::FromHex};
use alloy_provider::Provider;
use axum::{Json, extract::Query, extract::State};
use reth_chainspec::NamedChain;

pub async fn root_handler() -> &'static str {
    concat!(
        "Welcome to the pectralizer api v",
        env!("CARGO_PKG_VERSION"),
        "!"
    )
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
    let Some(block_hash) = receipt.block_hash else {
        return Err(HandlerError::BlockNotFound(tx_hash_bytes.to_string()));
    };
    let Some(block) = provider_state
        .ethereum_provider
        .get_block_by_hash(block_hash)
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get block by hash: {}", e)))?
    else {
        return Err(HandlerError::BlockNotFound(tx_hash_bytes.to_string()));
    };
    let timestamp = block.header.timestamp;
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
        // compute wei spent in different configurations
        let blob_data_wei_spent = blob_gas_used as u128 * blob_gas_price;
        let legacy_calldata_wei_spent = total_legacy_calldata_gas as u128 * gas_price;
        let eip_7623_calldata_wei_spent = total_eip_7623_calldata_gas as u128 * gas_price;
        Ok(TxAnalysisResponse {
            timestamp,
            blob_gas_used,
            gas_used,
            gas_price,
            blob_gas_price: Some(blob_gas_price),
            legacy_calldata_gas: total_legacy_calldata_gas,
            eip_7623_calldata_gas: total_eip_7623_calldata_gas,
            blob_data_wei_spent: Some(blob_data_wei_spent),
            legacy_calldata_wei_spent,
            eip_7623_calldata_wei_spent,
        })
    } else {
        let blob_gas_price = block.header.blob_fee();
        // get calldata
        let calldata = tx.input();
        // compute EIP-7623 calldata gas
        let eip_7623_calldata_gas = compute_calldata_gas(calldata);
        // compute legacy calldata gas
        let legacy_calldata_gas = compute_legacy_calldata_gas(calldata);
        // compute wei spent in different configurations
        let blob_data_wei_spent = if let Some(blob_gas_price) = blob_gas_price {
            // we need to compute the number of blobs needed to store the calldata
            // and then multiply by the blob gas price and the number of bytes in a blob
            let number_of_blobs_needed = calldata.len().div_ceil(BYTES_PER_BLOB as usize) as u128;
            Some(number_of_blobs_needed * BYTES_PER_BLOB as u128 * blob_gas_price)
        } else {
            None
        };
        let legacy_calldata_wei_spent = legacy_calldata_gas as u128 * gas_price;
        let eip_7623_calldata_wei_spent = eip_7623_calldata_gas as u128 * gas_price;
        Ok(TxAnalysisResponse {
            timestamp,
            blob_gas_used: 0,
            gas_used,
            gas_price,
            blob_gas_price,
            eip_7623_calldata_gas,
            legacy_calldata_gas,
            blob_data_wei_spent,
            legacy_calldata_wei_spent,
            eip_7623_calldata_wei_spent,
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
        .map_err(|_| HandlerError::InvalidHex(query.contract_address.clone()))?;
    // if EOA, return error
    if provider_state
        .ethereum_provider
        .get_code_at(contract_address)
        .await
        .map_err(|_| HandlerError::ProviderError("Failed to get code at".to_string()))?
        .is_empty()
    {
        return Err(HandlerError::InvalidContract(query.contract_address));
    }
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
    let mut influenced = 0;
    for tx_hash in &tx_hashes {
        let tx_analysis = analyze_transaction(&provider_state, *tx_hash).await?;
        if tx_analysis.gas_used == tx_analysis.eip_7623_calldata_gas + BASE_STIPEND {
            // tx is influenced by eip7623
            influenced += 1;
        }
    }
    Ok(Json(ContractAnalysisResponse {
        tx_list: tx_hashes,
        influenced,
    }))
}
