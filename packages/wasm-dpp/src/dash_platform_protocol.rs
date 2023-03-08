use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::document_facade::DocumentFacadeWasm;
use crate::fetch_and_validate_data_contract::DataContractFetcherAndValidatorWasm;
use crate::identity_facade::IdentityFacadeWasm;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::{with_js_error, DataContractFacadeWasm};
use crate::{DocumentFactoryWASM, DocumentValidatorWasm};
use dpp::identity::validation::PublicKeysValidator;

use crate::state_transition_facade::StateTransitionFacadeWasm;
use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};

use serde::{Deserialize, Serialize};

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol {
    identity: IdentityFacadeWasm,
    document: DocumentFacadeWasm,
    data_contract: DataContractFacadeWasm,
    state_transition: StateTransitionFacadeWasm,
}

#[derive(Serialize, Deserialize)]
pub struct DPPOptions {
    #[serde(rename = "protocolVersion")]
    pub current_protocol_version: Option<u32>,
}

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: JsValue,
        bls_adapter: JsBlsAdapter,
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<DashPlatformProtocol, JsValue> {
        // TODO: wrap whole thing around rs-dpp/dash_platform_protocol?
        let options: DPPOptions = with_js_error!(serde_wasm_bindgen::from_value(options))?;
        let wrapped_state_repository =
            ExternalStateRepositoryLikeWrapper::new(state_repository.clone());
        let bls = BlsAdapter(bls_adapter.clone());

        let protocol_version = options.current_protocol_version.unwrap_or(LATEST_VERSION);
        let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
            protocol_version,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        ));
        let public_keys_validator = Arc::new(PublicKeysValidator::new(bls).unwrap());

        let identity_facade =
            IdentityFacadeWasm::new(protocol_version_validator.clone(), public_keys_validator);

        let document_facade = init_document_facade(
            protocol_version,
            protocol_version_validator.clone(),
            wrapped_state_repository,
        );

        let data_contract_facade =
            DataContractFacadeWasm::new(protocol_version, protocol_version_validator.clone());

        let state_transition_facade = StateTransitionFacadeWasm::new(
            state_repository,
            bls_adapter,
            protocol_version_validator,
        )?;

        Ok(Self {
            document: document_facade,
            identity: identity_facade,
            data_contract: data_contract_facade,
            state_transition: state_transition_facade,
        })
    }

    #[wasm_bindgen(getter = dataContract)]
    pub fn data_contract(&self) -> DataContractFacadeWasm {
        self.data_contract.clone()
    }

    #[wasm_bindgen(getter=document)]
    pub fn document(&self) -> DocumentFacadeWasm {
        self.document.clone()
    }

    #[wasm_bindgen(getter = identity)]
    pub fn identity(&self) -> IdentityFacadeWasm {
        self.identity.clone()
    }

    #[wasm_bindgen(getter = stateTransition)]
    pub fn state_transition(&self) -> StateTransitionFacadeWasm {
        self.state_transition.clone()
    }
}

fn init_document_facade(
    protocol_version: u32,
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    state_repository: ExternalStateRepositoryLikeWrapper,
) -> DocumentFacadeWasm {
    let document_validator = DocumentValidatorWasm::new_with_arc(protocol_version_validator);

    let document_factory = DocumentFactoryWASM::new_with_state_repository_wrapper(
        protocol_version,
        document_validator.clone(),
        state_repository.clone(),
    );

    let data_contract_fetcher_and_validator =
        DataContractFetcherAndValidatorWasm::new_with_state_repository_wrapper(state_repository);

    DocumentFacadeWasm::new(
        document_validator,
        document_factory,
        data_contract_fetcher_and_validator,
    )
}
