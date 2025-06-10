use alloy_primitives::TxHash;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

/// Query parameters for the tx handler.
#[derive(Deserialize, Debug)]
pub struct TxHashQuery {
    /// The transaction hash to analyze.
    pub tx_hash: String,
}

/// Response structure for the tx handler.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct TxAnalysisResponse {
    /// The timestamp of the transaction.
    pub timestamp: u64,
    /// Total gas used by the transaction.
    pub gas_used: u64,
    /// Gas price used by the transaction.
    pub gas_price: u128,
    /// Blob gas price used by the transaction.
    ///
    /// None if the transaction happened before Cancun hard fork.
    pub blob_gas_price: Option<u128>,
    /// Blob gas used by the transaction.
    pub blob_gas_used: u64,
    /// EIP-7623 calldata gas.
    pub eip_7623_calldata_gas: u64,
    /// Legacy calldata gas.
    pub legacy_calldata_gas: u64,
    /// Blob data wei spent.
    ///
    /// None if the transaction happened before Cancun hard fork.
    pub blob_data_wei_spent: Option<u128>,
    /// Legacy calldata wei spent.
    pub legacy_calldata_wei_spent: u128,
    /// EIP-7623 calldata wei spent.
    pub eip_7623_calldata_wei_spent: u128,
}

/// Query parameters for the contract handler.
#[derive(Deserialize, Debug)]
pub struct ContractQuery {
    /// The contract address to analyze.
    pub contract_address: String,
}

/// Response structure for the contract handler.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct ContractAnalysisResponse {
    /// The list of transactions hash included in the analysis.
    pub tx_list: FxHashSet<TxHash>,
    /// The list of transactions hash influenced by EIP-7623.
    pub influenced_tx_list: Vec<TxHash>,
    /// The number of transactions influenced by EIP-7623.
    pub influenced: u64,
}

/// Query parameters for daily transactions endpoint.
#[derive(Deserialize, Debug)]
pub struct DailyTxsQuery {
    /// The batcher address to filter by.
    pub batcher_address: String,
    /// Timestamp start (Unix timestamp).
    pub start_timestamp: i64,
    /// Timestamp end (Unix timestamp).
    pub end_timestamp: i64,
}

/// Response structure for daily transactions endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct DailyTxsResponse {
    /// The batcher address.
    pub batcher_address: String,
    /// The number of transactions.
    pub tx_count: u64,
}

/// Query parameters for ETH saved endpoint.
#[derive(Deserialize, Debug)]
pub struct EthSavedQuery {
    /// The batcher address to filter by.
    pub batcher_address: String,
    /// Timestamp start (Unix timestamp).
    pub start_timestamp: i64,
    /// Timestamp end (Unix timestamp).
    pub end_timestamp: i64,
}

/// Response structure for ETH saved endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct EthSavedResponse {
    /// The batcher address.
    pub batcher_address: String,
    /// Total ETH saved in wei.
    pub total_eth_saved_wei: u128,
}

/// Query parameters for gas usage endpoints.
#[derive(Deserialize, Debug)]
pub struct GasUsageQuery {
    /// The batcher address to filter by.
    pub batcher_address: String,
    /// Timestamp start (Unix timestamp).
    pub start_timestamp: i64,
    /// Timestamp end (Unix timestamp).
    pub end_timestamp: i64,
}

/// Response structure for blob data gas endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BlobDataGasResponse {
    /// The batcher address.
    pub batcher_address: String,
    /// Total blob data gas used.
    pub total_blob_data_gas: u64,
}

/// Response structure for Pectra data gas endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct PectraDataGasResponse {
    /// The batcher address.
    pub batcher_address: String,
    /// Total Pectra (EIP-7623) calldata gas used.
    pub total_pectra_data_gas: u64,
}

/// Query parameters for aggregated endpoints (all batchers).
#[derive(Deserialize, Debug)]
pub struct AggregatedQuery {
    /// Timestamp start (Unix timestamp).
    pub start_timestamp: i64,
    /// Timestamp end (Unix timestamp).
    pub end_timestamp: i64,
}

/// Individual batcher data for daily transactions.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BatcherDailyTxs {
    /// The batcher address.
    pub batcher_address: String,
    /// The number of transactions.
    pub tx_count: u64,
}

/// Response structure for aggregated daily transactions endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AllDailyTxsResponse {
    /// List of batcher transaction data.
    pub batchers: Vec<BatcherDailyTxs>,
}

/// Individual batcher data for ETH saved.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BatcherEthSaved {
    /// The batcher address.
    pub batcher_address: String,
    /// Total ETH saved in wei.
    pub total_eth_saved_wei: u128,
}

/// Response structure for aggregated ETH saved endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AllEthSavedResponse {
    /// List of batcher ETH saved data.
    pub batchers: Vec<BatcherEthSaved>,
}

/// Individual batcher data for blob data gas.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BatcherBlobDataGas {
    /// The batcher address.
    pub batcher_address: String,
    /// Total blob data gas used.
    pub total_blob_data_gas: u64,
}

/// Response structure for aggregated blob data gas endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AllBlobDataGasResponse {
    /// List of batcher blob data gas.
    pub batchers: Vec<BatcherBlobDataGas>,
}

/// Individual batcher data for Pectra data gas.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BatcherPectraDataGas {
    /// The batcher address.
    pub batcher_address: String,
    /// Total Pectra (EIP-7623) calldata gas used.
    pub total_pectra_data_gas: u64,
}

/// Response structure for aggregated Pectra data gas endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AllPectraDataGasResponse {
    /// List of batcher Pectra data gas.
    pub batchers: Vec<BatcherPectraDataGas>,
}

/// Snapshot of daily aggregated metrics per batcher (previous 24-hour window).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyBatcherStats {
    /// Batcher address (lower-cased hex).
    pub batcher_address: String,
    /// Start of the 24-h period being summarised (Unix timestamp, UTC, aligned at midnight).
    pub snapshot_timestamp: i64,
    /// Total ETH saved in wei during the period.
    pub total_eth_saved_wei: u128,
    /// Total transactions in the period.
    pub total_daily_txs: u64,
    /// Total blob-data gas used.
    pub total_blob_data_gas: u64,
    /// Total Pectra (EIP-7623) calldata gas used.
    pub total_pectra_data_gas: u64,
}

/// Recent daily statistics (series) for a batcher.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct BatcherSevenDayStats {
    /// Batcher address (lower-cased hex).
    pub batcher_address: String,
    /// Timestamps of the snapshots (Unix timestamp, UTC, aligned at midnight).
    pub timestamps: Vec<i64>,
    /// Total transactions in the period.
    pub total_daily_txs: Vec<u64>,
    /// Total ETH saved in wei during the period.
    pub total_eth_saved_wei: Vec<u128>,
    /// Total blob-data gas used.
    pub total_blob_data_gas: Vec<u64>,
    /// Total Pectra (EIP-7623) calldata gas used.
    pub total_pectra_data_gas: Vec<u64>,
}

/// Response for the 7-day stats endpoint.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AllBatchersSevenDayStatsResponse {
    /// List of batcher seven-day stats.
    pub batchers: Vec<BatcherSevenDayStats>,
}
