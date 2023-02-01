use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::document_facade::DocumentFacadeWasm;
use crate::fetch_and_validate_data_contract::DataContractFetcherAndValidatorWasm;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::version::ProtocolVersionValidatorWasm;
use crate::{DocumentFactoryWASM, DocumentValidatorWasm};
use dpp::identity::validation::PublicKeysValidator;
use dpp::identity::IdentityFacade;
use dpp::version::ProtocolVersionValidator;

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocol {
    _identity: IdentityFacade<BlsAdapter>,
    document: DocumentFacadeWasm,
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
        // TODO: remove default validator and make a real one instead
        let validator = Arc::new(ProtocolVersionValidator::default());

        let public_keys_validator = PublicKeysValidator::new(bls).unwrap();
        let identity_facade =
            IdentityFacade::new(validator.clone(), Arc::new(public_keys_validator)).unwrap();

        let protocol_version = validator.protocol_version();
        let protocol_version_validator: ProtocolVersionValidatorWasm =
            ProtocolVersionValidator::default().into();

        let document_validator = DocumentValidatorWasm::new(protocol_version_validator);
        let document_factory = DocumentFactoryWASM::new_with_state_repository_wrapper(
            protocol_version,
            document_validator.clone(),
            wrapped_state_repository.clone(),
        );
        let data_contract_fetcher_and_validator =
            DataContractFetcherAndValidatorWasm::new_with_state_repository_wrapper(
                wrapped_state_repository,
            );

        let document_facade = DocumentFacadeWasm::new(
            document_validator,
            document_factory,
            data_contract_fetcher_and_validator,
        );

        DashPlatformProtocol {
            _identity: identity_facade,
            document: document_facade,
        }

        // DashPlatformProtocol(identity_facade)
    }
}
