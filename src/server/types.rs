use serde::{Deserialize, Serialize};

/// Query parameters for the tx handler.
#[derive(Deserialize)]
pub struct TxHashQuery {
    /// The transaction hash to analyze.
    pub tx_hash: String,
}

/// Response structure for the tx handler.
#[derive(Serialize)]
pub struct TxAnalysisResponse {
    /// Whether EIP-7623 is effective.
    pub is_eip_7623_effective: bool,
    /// Total gas used.
    pub gas_used: u64,
    /// EIP-7623 calldata gas.
    pub eip_7623_calldata_gas: u64,
}
