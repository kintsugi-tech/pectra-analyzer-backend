use crate::{provider::ProviderState, tracker::database::Database};
use std::sync::Arc;

/// Shared application state containing provider and database.
#[derive(Clone)]
pub struct AppState {
    pub provider_state: ProviderState,
    pub db: Arc<dyn Database>,
}

pub mod error;
pub mod handlers;
pub mod types;
