use std::sync::Arc;

use wasm_bindgen::prelude::*;

use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;
use dpp::version::ProtocolVersionValidator;

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol(IdentityFacade);

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new() -> DashPlatformProtocol {
        // TODO: remove default validator and make a real one instead
        let validator = ProtocolVersionValidator::default();
        let public_keys_validator = PublicKeysValidator::new().unwrap();
        let identity_facade =
            IdentityFacade::new(Arc::new(validator), Arc::new(public_keys_validator)).unwrap();

        DashPlatformProtocol(identity_facade)
    }
}

impl Default for DashPlatformProtocol {
    fn default() -> Self {
        Self::new()
    }
}
