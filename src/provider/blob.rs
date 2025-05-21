use alloy_chains::NamedChain;
use alloy_primitives::Bytes;
use reqwest::Client;
use serde::Deserialize;

/// The url of the blob provider, aka blobscan.
const MAINNET_BLOB_PROVIDER_URL: &str = "https://api.blobscan.com/blobs/";
const SEPOLIA_BLOB_PROVIDER_URL: &str = "https://api.sepolia.blobscan.com/blobs/";

/// The data of the blob.
#[derive(Debug, Deserialize)]
pub struct BlobData {
    /// The data field of the blob.
    pub data: Bytes,
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
            println!("We don't support this chain id for the blob provider, fallback to mainnet");
            MAINNET_BLOB_PROVIDER_URL
        };
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
            chain_id,
        }
    }

    /// Make a blob request to the provider providing the blob versioned hash.
    pub async fn get_blob_data(&self, blob_versioned_hash: &str) -> eyre::Result<BlobData> {
        let url = format!("{}{}", self.endpoint, blob_versioned_hash);
        let response = self.client.get(url).send().await?;
        let blob_data: BlobData = response.json().await?;
        Ok(blob_data)
    }
}
