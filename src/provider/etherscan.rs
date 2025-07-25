use alloy_primitives::{Address, TxHash};
use reqwest::Client;
use serde::Deserialize;

/// The etherscan base endpoint url.
const ETHERSCAN_ENDPOINT: &str = "https://api.etherscan.io/v2/api";

/// Etherscan response.
#[derive(Debug, Deserialize)]
pub struct EtherscanResponse {
    /// The result of the etherscan response.
    pub result: Vec<EtherscanTx>,
}

/// Etherscan transaction.
#[derive(Debug, Deserialize)]
pub struct EtherscanTx {
    /// The hash of the transaction.
    pub hash: TxHash,
}

/// The etherscan provider.
#[derive(Debug)]
pub struct EtherscanProvider {
    /// The reqwest client to handle connections to the etherscan provider.
    pub client: Client,
    /// The etherscan api key.
    pub api_key: String,
    /// The etherscan endpoint url.
    pub endpoint: String,
    /// The chain id.
    pub chain_id: u64,
}

impl EtherscanProvider {
    /// Create a new etherscan provider.
    pub fn new(api_key: String, chain_id: u64) -> Self {
        Self {
            client: Client::new(),
            api_key,
            endpoint: ETHERSCAN_ENDPOINT.to_string(),
            chain_id,
        }
    }
}

impl EtherscanProvider {
    /// Get last (offset) internal transactions of an address.
    pub async fn get_internal_txs(
        &self,
        address: Address,
        start_block: u64,
        end_block: u64,
        offset: u64,
    ) -> eyre::Result<EtherscanResponse> {
        let url = format!(
            "{}?chainid={}&module=account&action=txlistinternal&address={}&startblock={}&endblock={}&page=1&offset={}&sort=asc&apikey={}",
            self.endpoint, self.chain_id, address, start_block, end_block, offset, self.api_key,
        );
        let response = self.client.get(url).send().await?;
        let txs: EtherscanResponse = response.json().await?;
        Ok(txs)
    }

    /// Get last (offset) normal transactions of an address.
    pub async fn get_normal_txs(
        &self,
        address: Address,
        start_block: u64,
        end_block: u64,
        offset: u64,
    ) -> eyre::Result<EtherscanResponse> {
        let url = format!(
            "{}?chainid={}&module=account&action=txlist&address={}&startblock={}&endblock={}&page=1&offset={}&sort=asc&apikey={}",
            self.endpoint, self.chain_id, address, start_block, end_block, offset, self.api_key,
        );
        let response = self.client.get(url).send().await?;
        let txs: EtherscanResponse = response.json().await?;
        Ok(txs)
    }
}
