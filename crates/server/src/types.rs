use core::types::PlatformsFundingRate;
use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct AppState {
    pub platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
}
