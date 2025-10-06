use std::sync::Arc;

use crate::protocol::JsonRpcTranslator;
use crate::services::{CoreServiceImpl, PlatformServiceImpl};

#[derive(Clone)]
pub(super) struct JsonRpcAppState {
    pub platform_service: PlatformServiceImpl,
    pub core_service: CoreServiceImpl,
    pub translator: Arc<JsonRpcTranslator>,
}

#[derive(Clone)]
pub(super) struct MetricsAppState {
    pub platform_service: PlatformServiceImpl,
    pub core_service: CoreServiceImpl,
}
