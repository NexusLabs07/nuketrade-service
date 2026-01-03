use core::types::PlatformsFundingRate;
use std::sync::Arc;

use arc_swap::ArcSwap;

#[derive(Clone, Debug)]
pub struct AppState {
    pub platforms_funding_rate: Arc<ArcSwap<PlatformsFundingRate>>,
}
