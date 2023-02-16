use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::identity_facade::IdentityFacadeWasm;
use crate::DataContractFacadeWasm;
use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;
use dpp::version::{ProtocolVersionValidator, LATEST_VERSION};

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol {
    identity: IdentityFacadeWasm,
    data_contract: DataContractFacadeWasm,
}

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new(bls_adapter: JsBlsAdapter) -> Self {
        // TODO: add protocol version to the constructor
        let protocol_version = LATEST_VERSION;
        let bls = BlsAdapter(bls_adapter);
        // TODO: remove default validator and make a real one instead
        let validator = ProtocolVersionValidator::default();
        let protocol_version_validator = Arc::new(validator);
        let public_keys_validator = Arc::new(PublicKeysValidator::new(bls.clone()).unwrap());

        let identity_facade =
            IdentityFacadeWasm::new(protocol_version_validator.clone(), public_keys_validator);

        let data_contract_facade =
            DataContractFacadeWasm::new(protocol_version, protocol_version_validator);

        Self {
            identity: identity_facade,
            data_contract: data_contract_facade,
        }
    }

    #[wasm_bindgen(getter = dataContract)]
    pub fn data_contract(&self) -> DataContractFacadeWasm {
        self.data_contract.clone()
    }

    #[wasm_bindgen(getter = identity)]
    pub fn identity(&self) -> IdentityFacadeWasm {
        self.identity.clone()
    }
}
