use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;
use dpp::version::ProtocolVersionValidator;

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol(IdentityFacade<BlsAdapter>);

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new(bls_adapter: JsBlsAdapter) -> DashPlatformProtocol {
        let bls = BlsAdapter(bls_adapter);
        // TODO: remove default validator and make a real one instead
        let validator = ProtocolVersionValidator::default();
        let public_keys_validator = PublicKeysValidator::new(bls).unwrap();
        let identity_facade =
            IdentityFacade::new(Arc::new(validator), Arc::new(public_keys_validator)).unwrap();

        DashPlatformProtocol(identity_facade)
    }
}
