use alloy_primitives::Bytes;
use reqwest::Client;
use serde::Deserialize;

/// The url of the blob provider, aka blobscan.
const BLOB_PROVIDER_URL: &str = "https://api.blobscan.com/blobs/";

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
}

impl BlobProvider {
    /// Create a new blob provider.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            endpoint: BLOB_PROVIDER_URL.to_string(),
        }
    }

    /// Make a blob request to the provider providing the blob versioned hash.
    pub async fn blob_data(&self, blob_versioned_hash: &str) -> eyre::Result<BlobData> {
        let url = format!("{}{}", self.endpoint, blob_versioned_hash);
        let response = self.client.get(url).send().await?;
        let blob_data: BlobData = response.json().await?;
        Ok(blob_data)
    }
}

impl Default for BlobProvider {
    fn default() -> Self {
        Self::new()
    }
}
