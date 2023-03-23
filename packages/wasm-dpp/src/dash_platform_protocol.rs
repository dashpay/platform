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
    protocol_version: u32,
    state_repository: ExternalStateRepositoryLike,
    public_keys_validator: Arc<PublicKeysValidator<BlsAdapter>>,
    bls: BlsAdapter,
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
        bls_adapter: JsBlsAdapter,
        state_repository: ExternalStateRepositoryLike,
        maybe_protocol_version: Option<u32>,
    ) -> Result<DashPlatformProtocol, JsValue> {
        let bls = BlsAdapter(bls_adapter.clone());
        let protocol_version = maybe_protocol_version.unwrap_or(LATEST_VERSION);
        let public_keys_validator = Arc::new(PublicKeysValidator::new(bls.clone()).unwrap());

        let (identity_facade, document_facade, data_contract_facade, state_transition_facade) =
            create_facades(
                public_keys_validator.clone(),
                protocol_version,
                state_repository.clone(),
                bls.clone(),
            )?;

        Ok(Self {
            document: document_facade,
            identity: identity_facade,
            data_contract: data_contract_facade,
            state_transition: state_transition_facade,
            protocol_version,
            state_repository,
            public_keys_validator,
            bls,
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

    #[wasm_bindgen(getter = protocolVersion)]
    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    #[wasm_bindgen(js_name = getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.protocol_version()
    }

    #[wasm_bindgen(js_name = setProtocolVersion)]
    pub fn set_protocol_version(&mut self, protocol_version: u32) -> Result<(), JsValue> {
        self.init(
            protocol_version,
            self.state_repository.clone(),
            self.bls.clone(),
        )
    }

    #[wasm_bindgen(js_name = setStateRepository)]
    pub fn set_state_repository(
        &mut self,
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<(), JsValue> {
        self.init(self.protocol_version, state_repository, self.bls.clone())
    }

    #[wasm_bindgen(js_name = getStateRepository)]
    pub fn get_state_repository(&self) -> ExternalStateRepositoryLike {
        self.state_repository.clone()
    }

    fn init(
        &mut self,
        protocol_version: u32,
        state_repository: ExternalStateRepositoryLike,
        bls_adapter: BlsAdapter,
    ) -> Result<(), JsValue> {
        let (identity_facade, document_facade, data_contract_facade, state_transition_facade) =
            create_facades(
                self.public_keys_validator.clone(),
                protocol_version,
                state_repository.clone(),
                bls_adapter,
            )?;

        self.protocol_version = protocol_version;
        self.identity = identity_facade;
        self.document = document_facade;
        self.data_contract = data_contract_facade;
        self.state_transition = state_transition_facade;
        self.state_repository = state_repository;

        Ok(())
    }
}

fn create_facades(
    public_keys_validator: Arc<PublicKeysValidator<BlsAdapter>>,
    protocol_version: u32,
    state_repository: ExternalStateRepositoryLike,
    bls_adapter: BlsAdapter,
) -> Result<
    (
        IdentityFacadeWasm,
        DocumentFacadeWasm,
        DataContractFacadeWasm,
        StateTransitionFacadeWasm,
    ),
    JsValue,
> {
    let wrapped_state_repository =
        ExternalStateRepositoryLikeWrapper::new(state_repository.clone());
    let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
        protocol_version,
        LATEST_VERSION,
        COMPATIBILITY_MAP.clone(),
    ));

    let identity_facade =
        IdentityFacadeWasm::new(protocol_version_validator.clone(), public_keys_validator);

    let document_facade = init_document_facade(
        protocol_version,
        protocol_version_validator.clone(),
        wrapped_state_repository.clone(),
    );

    let data_contract_facade =
        DataContractFacadeWasm::new(protocol_version, protocol_version_validator.clone());

    let state_transition_facade =
        StateTransitionFacadeWasm::new(state_repository, bls_adapter, protocol_version_validator)?;

    Ok((
        identity_facade,
        document_facade,
        data_contract_facade,
        state_transition_facade,
    ))
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
