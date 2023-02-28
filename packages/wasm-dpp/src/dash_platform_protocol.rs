use std::sync::Arc;

use dpp::identity::IdentityFacade;
use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::document_facade::DocumentFacadeWasm;
use crate::fetch_and_validate_data_contract::DataContractFetcherAndValidatorWasm;
use crate::identity_facade::IdentityFacadeWasm;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::version::ProtocolVersionValidatorWasm;
use crate::DataContractFacadeWasm;
use crate::{DocumentFactoryWASM, DocumentValidatorWasm};
use dpp::identity::validation::PublicKeysValidator;

use dpp::version::{ProtocolVersionValidator, LATEST_VERSION};

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol {
    identity: IdentityFacade<BlsAdapter>,
    document: DocumentFacadeWasm,
    data_contract: DataContractFacadeWasm,
}

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocol {
    #[wasm_bindgen(constructor)]
    pub fn new(
        bls_adapter: JsBlsAdapter,
        state_repository: ExternalStateRepositoryLike,
    ) -> DashPlatformProtocol {
        let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
        let bls = BlsAdapter(bls_adapter);

        // TODO: add protocol version to the constructor
        let protocol_version = LATEST_VERSION;
        // TODO: remove default validator and make a real one instead
        let protocol_version_validator = ProtocolVersionValidator::default();
        let protocol_version_validator_wasm: ProtocolVersionValidatorWasm =
            (&protocol_version_validator).into();
        let protocol_version_validator_arc = Arc::new(protocol_version_validator);

        let public_keys_validator = PublicKeysValidator::new(bls).unwrap();

        let identity_facade = IdentityFacade::new(
            protocol_version_validator_arc.clone(),
            Arc::new(public_keys_validator),
        )
        .unwrap();

        // initialization of Document
        let document_validator = DocumentValidatorWasm::new(protocol_version_validator_wasm);
        let document_factory = DocumentFactoryWASM::new_with_state_repository_wrapper(
            protocol_version,
            document_validator.clone(),
            wrapped_state_repository.clone(),
        );
        let data_contract_fetcher_and_validator =
            DataContractFetcherAndValidatorWasm::new_with_state_repository_wrapper(
                wrapped_state_repository,
            );
        let document_facade = DocumentFacadeWasm::new_with_arc(
            Arc::new(document_validator),
            Arc::new(document_factory),
            Arc::new(data_contract_fetcher_and_validator),
        );

        let data_contract_facade =
            DataContractFacadeWasm::new(protocol_version, protocol_version_validator_arc);

        Self {
            document: document_facade,
            identity: identity_facade,
            data_contract: data_contract_facade,
        }
    }

    #[wasm_bindgen(getter = dataContract)]
    pub fn data_contract(&self) -> DataContractFacadeWasm {
        self.data_contract.clone()
    }

    #[wasm_bindgen(getter=document)]
    pub fn get_document(&self) -> DocumentFacadeWasm {
        self.document.clone()
    }

    #[wasm_bindgen(getter = identity)]
    pub fn identity(&self) -> IdentityFacadeWasm {
        self.identity.clone()
    }
}
