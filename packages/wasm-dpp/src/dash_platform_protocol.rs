use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::DataContractFacadeWasm;
use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;
use dpp::version::ProtocolVersionValidator;

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol {
    identity: IdentityFacade<BlsAdapter>,
    data_contract: DataContractFacadeWasm,
}

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new(bls_adapter: JsBlsAdapter) -> Self {
        // TODO: add protocol version to the constructor
        let protocol_version = 0;
        let bls = BlsAdapter(bls_adapter);
        // TODO: remove default validator and make a real one instead
        let validator = ProtocolVersionValidator::default();
        let protocol_version_validator = Arc::new(validator);
        let public_keys_validator = PublicKeysValidator::new(bls).unwrap();
        let identity_facade = IdentityFacade::new(
            protocol_version_validator.clone(),
            Arc::new(public_keys_validator),
        )
        .unwrap();

        let data_contract_facade =
            DataContractFacadeWasm::new(protocol_version, protocol_version_validator.clone());

        Self {
            identity: identity_facade,
            data_contract: data_contract_facade,
        }
    }

    #[wasm_bindgen(getter = dataContract)]
    pub fn data_contract(&self) -> DataContractFacadeWasm {
        self.data_contract.clone()
    }
}
