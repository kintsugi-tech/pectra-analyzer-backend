use alloy_chains::NamedChain;
use alloy_primitives::TxHash;
use reqwest::Client;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tracing::warn;

/// The url of the blob provider, aka blobscan.
const MAINNET_BLOB_PROVIDER_URL: &str = "https://api.blobscan.com/transactions/";
const SEPOLIA_BLOB_PROVIDER_URL: &str = "https://api.sepolia.blobscan.com/transactions/";

/// Custom deserializer to convert string to u64
fn deserialize_string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(D::Error::custom)
}

/// The data of the blob.
#[derive(Debug, Deserialize)]
pub struct BlobData {
    /// The amount of gas used that would have been used to store the blob data as calldata.
    #[serde(
        rename = "blobAsCalldataGasUsed",
        deserialize_with = "deserialize_string_to_u64"
    )]
    pub blob_as_calldata_gas_used: u64,
}

/// The provider of the blobs.
#[derive(Debug)]
pub struct BlobProvider {
    /// The reqwest client to handle connections to the blob provider.
    pub client: Client,
    /// The blob provider endpoint url.
    pub endpoint: String,
    /// The chain id.
    pub chain_id: u64,
}

impl BlobProvider {
    /// Create a new blob provider.
    pub fn new(chain_id: u64) -> Self {
        let endpoint = if chain_id == <NamedChain as Into<u64>>::into(NamedChain::Mainnet) {
            MAINNET_BLOB_PROVIDER_URL
        } else if chain_id == <NamedChain as Into<u64>>::into(NamedChain::Sepolia) {
            SEPOLIA_BLOB_PROVIDER_URL
        } else {
            warn!("We don't support this chain id for the blob provider, fallback to mainnet");
            MAINNET_BLOB_PROVIDER_URL
        };
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
            chain_id,
        }
    }

    /// Make a blob request to the provider providing the transaction hash.
    pub async fn get_blob_data(&self, tx_hash: &TxHash) -> eyre::Result<BlobData> {
        let url = format!("{}{}", self.endpoint, tx_hash);
        let response = self.client.get(url).send().await?;
        let blob_data: BlobData = response.json().await?;
        Ok(blob_data)
    }
}
