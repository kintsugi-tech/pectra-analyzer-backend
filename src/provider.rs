use crate::provider::blob_provider::BlobProvider;
use alloy_provider::RootProvider;
use std::sync::Arc;

pub mod blob_provider;

/// Shared state for the application that contains the providers
#[derive(Clone)]
pub struct ProviderState {
    /// The Ethereum provider
    pub ethereum_provider: Arc<RootProvider>,
    /// The blob provider
    pub blob_provider: Arc<BlobProvider>,
}

impl ProviderState {
    /// Create a new provider state with the given Ethereum provider URL
    pub async fn new(ethereum_provider_url: &str) -> Self {
        let ethereum_provider = RootProvider::connect(ethereum_provider_url).await.unwrap();
        Self {
            ethereum_provider: Arc::new(ethereum_provider),
            blob_provider: Arc::new(BlobProvider::default()),
        }
    }
}
