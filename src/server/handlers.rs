use super::{
    error::HandlerError,
    types::{
        AggregatedQuery, AllBlobDataGasResponse, AllDailyTxsResponse, AllEthSavedResponse,
        AllPectraDataGasResponse, BlobDataGasResponse, ContractAnalysisResponse, ContractQuery,
        DailyTxsQuery, DailyTxsResponse, EthSavedQuery, EthSavedResponse, GasUsageQuery,
        PectraDataGasResponse, TxAnalysisResponse, TxHashQuery,
    },
};
use crate::{
    provider::ProviderState,
    server::types::{AllBatchersSevenDayStatsResponse, BatcherSevenDayStats},
    utils::{
        BASE_STIPEND, BYTES_PER_BLOB, ISTANBUL_BLOCK_NUMBER, compute_calldata_gas,
        compute_legacy_calldata_gas,
    },
};
use alloy_consensus::{Transaction, Typed2718};
use alloy_primitives::{Address, FixedBytes, hex::FromHex};
use alloy_provider::Provider;
use axum::{Json, extract::Query, extract::State};
use rustc_hash::FxHashSet;
use std::collections::HashMap;

pub async fn root_handler() -> &'static str {
    concat!(
        "Welcome to the pectralizer api v",
        env!("CARGO_PKG_VERSION"),
        "!"
    )
}

pub async fn analyze_transaction(
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
        // get blob data from blobscan
        let total_legacy_calldata_gas;
        let total_eip_7623_calldata_gas;
        let blob_data = provider_state
            .blob_provider
            .get_blob_data(&tx_hash_bytes)
            .await
            .map_err(|e| HandlerError::ProviderError(format!("Failed to get blob data: {}", e)))?;
        if block.header.number < ISTANBUL_BLOCK_NUMBER {
            // pre Pectra, so pre EIP-7623
            total_legacy_calldata_gas = blob_data.blob_as_calldata_gas_used;
            total_eip_7623_calldata_gas =
                total_legacy_calldata_gas + (total_legacy_calldata_gas as f64 * 0.6) as u64;
        } else {
            // post Pectra, so post EIP-7623
            total_eip_7623_calldata_gas = blob_data.blob_as_calldata_gas_used;
            total_legacy_calldata_gas =
                total_eip_7623_calldata_gas - (total_eip_7623_calldata_gas as f64 * 0.6) as u64;
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
        let eip_7623_calldata_gas = compute_calldata_gas(calldata, block.header.number);
        // compute legacy calldata gas
        let legacy_calldata_gas = compute_legacy_calldata_gas(calldata, block.header.number);
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
    State(app_state): State<super::AppState>,
    Query(query): Query<TxHashQuery>,
) -> Result<Json<TxAnalysisResponse>, HandlerError> {
    // transform tx hash into a fixed bytes
    let tx_hash_bytes = FixedBytes::from_hex(&query.tx_hash)
        .map_err(|_| HandlerError::InvalidHex(query.tx_hash))?;
    let tx_analysis = analyze_transaction(&app_state.provider_state, tx_hash_bytes).await?;
    Ok(Json(tx_analysis))
}

pub async fn contract_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<ContractQuery>,
) -> Result<Json<ContractAnalysisResponse>, HandlerError> {
    let contract_address = Address::from_hex(&query.contract_address)
        .map_err(|_| HandlerError::InvalidHex(query.contract_address.clone()))?;
    // if EOA, return error
    if app_state
        .provider_state
        .ethereum_provider
        .get_code_at(contract_address)
        .await
        .map_err(|_| HandlerError::ProviderError("Failed to get code at".to_string()))?
        .is_empty()
    {
        return Err(HandlerError::InvalidContract(query.contract_address));
    }
    let last_block_number = app_state
        .provider_state
        .ethereum_provider
        .get_block_number()
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get block number: {}", e)))?;
    // 300 blocks are roughly 1 hour in Ethereum mainnet with 12s block time
    let start_block = last_block_number - 300;
    // collect all transaction hashes into a single Vec directly
    let mut tx_list = Vec::new();
    // get last (up to 5) internal transactions
    let internal_txs = app_state
        .provider_state
        .etherscan_provider
        .get_internal_txs(contract_address, start_block, last_block_number, 5)
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get internal txs: {}", e)))?;
    tx_list.extend(internal_txs.result.iter().map(|tx| tx.hash));
    // get last (up to 5) normal transactions
    let normal_txs = app_state
        .provider_state
        .etherscan_provider
        .get_normal_txs(contract_address, start_block, last_block_number, 5)
        .await
        .map_err(|e| HandlerError::ProviderError(format!("Failed to get normal txs: {}", e)))?;
    tx_list.extend(normal_txs.result.iter().map(|tx| tx.hash));
    let mut influenced = 0;
    let mut influenced_tx_list = Vec::with_capacity(tx_list.len());
    // deduplicate tx list
    let unique_tx_list: FxHashSet<_> = tx_list.into_iter().collect();
    for tx_hash in &unique_tx_list {
        let tx_analysis = analyze_transaction(&app_state.provider_state, *tx_hash).await?;
        if tx_analysis.gas_used == tx_analysis.eip_7623_calldata_gas + BASE_STIPEND {
            // tx is influenced by eip7623
            influenced += 1;
            influenced_tx_list.push(*tx_hash);
        }
    }
    Ok(Json(ContractAnalysisResponse {
        tx_list: unique_tx_list,
        influenced_tx_list,
        influenced,
    }))
}

/// Handler for daily transactions endpoint
pub async fn daily_txs_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<DailyTxsQuery>,
) -> Result<Json<DailyTxsResponse>, HandlerError> {
    let tx_count = app_state
        .db
        .get_daily_transactions(
            &query.batcher_address,
            query.start_timestamp,
            query.end_timestamp,
        )
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get daily transactions: {}", e))
        })?;

    Ok(Json(DailyTxsResponse {
        batcher_address: query.batcher_address,
        tx_count,
    }))
}

/// Handler for ETH saved endpoint
pub async fn eth_saved_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<EthSavedQuery>,
) -> Result<Json<EthSavedResponse>, HandlerError> {
    let total_eth_saved_wei = app_state
        .db
        .get_eth_saved_data(
            &query.batcher_address,
            query.start_timestamp,
            query.end_timestamp,
        )
        .await
        .map_err(|e| HandlerError::DatabaseError(format!("Failed to get ETH saved data: {}", e)))?;

    Ok(Json(EthSavedResponse {
        batcher_address: query.batcher_address,
        total_eth_saved_wei,
    }))
}

/// Handler for blob data gas endpoint
pub async fn blob_data_gas_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<GasUsageQuery>,
) -> Result<Json<BlobDataGasResponse>, HandlerError> {
    let total_blob_data_gas = app_state
        .db
        .get_total_blob_data_gas(
            &query.batcher_address,
            query.start_timestamp,
            query.end_timestamp,
        )
        .await
        .map_err(|e| HandlerError::DatabaseError(format!("Failed to get blob data gas: {}", e)))?;

    Ok(Json(BlobDataGasResponse {
        batcher_address: query.batcher_address,
        total_blob_data_gas,
    }))
}

/// Handler for Pectra data gas endpoint
pub async fn pectra_data_gas_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<GasUsageQuery>,
) -> Result<Json<PectraDataGasResponse>, HandlerError> {
    let total_pectra_data_gas = app_state
        .db
        .get_total_pectra_data_gas(
            &query.batcher_address,
            query.start_timestamp,
            query.end_timestamp,
        )
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get Pectra data gas: {}", e))
        })?;

    Ok(Json(PectraDataGasResponse {
        batcher_address: query.batcher_address,
        total_pectra_data_gas,
    }))
}

/// Handler for aggregated daily transactions endpoint (all batchers)
pub async fn all_daily_txs_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<AggregatedQuery>,
) -> Result<Json<AllDailyTxsResponse>, HandlerError> {
    let batchers = app_state
        .db
        .get_all_daily_transactions(query.start_timestamp, query.end_timestamp)
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get all daily transactions: {}", e))
        })?;

    Ok(Json(AllDailyTxsResponse { batchers }))
}

/// Handler for aggregated ETH saved endpoint (all batchers)
pub async fn all_eth_saved_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<AggregatedQuery>,
) -> Result<Json<AllEthSavedResponse>, HandlerError> {
    let batchers = app_state
        .db
        .get_all_eth_saved_data(query.start_timestamp, query.end_timestamp)
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get all ETH saved data: {}", e))
        })?;

    Ok(Json(AllEthSavedResponse { batchers }))
}

/// Handler for aggregated blob data gas endpoint (all batchers)
pub async fn all_blob_data_gas_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<AggregatedQuery>,
) -> Result<Json<AllBlobDataGasResponse>, HandlerError> {
    let batchers = app_state
        .db
        .get_all_total_blob_data_gas(query.start_timestamp, query.end_timestamp)
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get all blob data gas: {}", e))
        })?;

    Ok(Json(AllBlobDataGasResponse { batchers }))
}

/// Handler for aggregated Pectra data gas endpoint (all batchers)
pub async fn all_pectra_data_gas_handler(
    State(app_state): State<super::AppState>,
    Query(query): Query<AggregatedQuery>,
) -> Result<Json<AllPectraDataGasResponse>, HandlerError> {
    let batchers = app_state
        .db
        .get_all_total_pectra_data_gas(query.start_timestamp, query.end_timestamp)
        .await
        .map_err(|e| {
            HandlerError::DatabaseError(format!("Failed to get all Pectra data gas: {}", e))
        })?;

    Ok(Json(AllPectraDataGasResponse { batchers }))
}

pub async fn seven_day_stats_handler(
    State(app_state): State<super::AppState>,
) -> Result<Json<AllBatchersSevenDayStatsResponse>, HandlerError> {
    let rows = app_state.db.get_recent_daily_stats(7).await.map_err(|e| {
        HandlerError::DatabaseError(format!("Failed to get recent daily stats: {}", e))
    })?;

    let mut map: HashMap<String, BatcherSevenDayStats> = HashMap::new();

    for r in rows {
        let entry = map
            .entry(r.batcher_address.clone())
            .or_insert_with(|| BatcherSevenDayStats {
                batcher_address: r.batcher_address.clone(),
                timestamps: Vec::new(),
                total_daily_txs: Vec::new(),
                total_eth_saved_wei: Vec::new(),
                total_blob_data_gas: Vec::new(),
                total_pectra_data_gas: Vec::new(),
            });
        entry.timestamps.push(r.snapshot_timestamp);
        entry.total_daily_txs.push(r.total_daily_txs);
        entry.total_eth_saved_wei.push(r.total_eth_saved_wei);
        entry.total_blob_data_gas.push(r.total_blob_data_gas);
        entry.total_pectra_data_gas.push(r.total_pectra_data_gas);
    }

    let mut batchers: Vec<BatcherSevenDayStats> = map.into_values().collect();
    // ensure ascending order by timestamp inside vectors (they are already since query sorted asc)
    batchers.sort_by(|a, b| a.batcher_address.cmp(&b.batcher_address));

    Ok(Json(AllBatchersSevenDayStatsResponse { batchers }))
}
