pub mod client;
pub mod message;
use std::sync::Arc;

pub use client::watch;

#[derive(Debug, Clone)]
pub struct AssetUpdatedMessage {
    pub asset_id: String,
    pub asset_data: Arc<Vec<u8>>,
}
