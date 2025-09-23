use std::sync::Arc;

use crate::protocol::{JsonRpcTranslator, RestTranslator};
use crate::services::{CoreServiceImpl, PlatformServiceImpl};

#[derive(Clone)]
pub(super) struct RestAppState {
    pub platform_service: PlatformServiceImpl,
    pub core_service: CoreServiceImpl,
    pub translator: Arc<RestTranslator>,
}

#[derive(Clone)]
pub(super) struct JsonRpcAppState {
    pub platform_service: PlatformServiceImpl,
    pub core_service: CoreServiceImpl,
    pub translator: Arc<JsonRpcTranslator>,
}
