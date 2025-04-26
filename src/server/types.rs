use serde::{Deserialize, Serialize};

/// Query parameters for the tx handler.
#[derive(Deserialize, Debug)]
pub struct TxHashQuery {
    /// The transaction hash to analyze.
    pub tx_hash: String,
}

/// Response structure for the tx handler.
#[derive(Serialize, Debug)]
pub struct TxAnalysisResponse {
    /// Total gas used by the transaction.
    pub gas_used: u64,
    /// EIP-7623 calldata gas.
    pub eip_7623_calldata_gas: u64,
}
