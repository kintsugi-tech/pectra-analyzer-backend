use crate::provider::blob::BlobProvider;
use alloy_provider::RootProvider;
use etherscan::EtherscanProvider;
use std::sync::Arc;

pub mod blob;
pub mod etherscan;

/// Shared state for the application that contains the providers
#[derive(Clone)]
pub struct ProviderState {
    /// The Ethereum provider
    pub ethereum_provider: Arc<RootProvider>,
    /// The blob provider
    pub blob_provider: Arc<BlobProvider>,
    /// The etherscan provider
    pub etherscan_provider: Arc<EtherscanProvider>,
}

impl ProviderState {
    /// Create a new provider state with the given Ethereum provider URL
    pub async fn new(ethereum_provider_url: &str, etherscan_api_key: &str, chain_id: u64) -> Self {
        let ethereum_provider = RootProvider::connect(ethereum_provider_url).await.unwrap();
        let etherscan_provider = EtherscanProvider::new(etherscan_api_key.to_string(), chain_id);
        Self {
            ethereum_provider: Arc::new(ethereum_provider),
            blob_provider: Arc::new(BlobProvider::new(chain_id)),
            etherscan_provider: Arc::new(etherscan_provider),
        }
    }
}
