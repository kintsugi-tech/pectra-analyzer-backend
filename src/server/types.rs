use alloy_primitives::TxHash;
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
    pub tx_list: Vec<TxHash>,
    /// The list of transactions hash influenced by EIP-7623.
    pub influenced_tx_list: Vec<TxHash>,
    /// The number of transactions influenced by EIP-7623.
    pub influenced: u64,
}
