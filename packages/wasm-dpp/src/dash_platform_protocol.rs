use wasm_bindgen::prelude::*;

use dpp::dash_platform_protocol::{DashPlatformProtocol, StateTransitionFactory};
use dpp::state_transition::StateTransition;
use dpp::version::LATEST_VERSION;

use crate::entropy_generator::ExternalEntropyGenerator;
use crate::identity::identity_facade::IdentityFacadeWasm;

#[wasm_bindgen(js_name=DashPlatformProtocol)]
pub struct DashPlatformProtocolWasm(DashPlatformProtocol);

#[wasm_bindgen(js_name=StateTransitionFactory)]
pub struct StateTransitionFactoryWasm(StateTransitionFactory);

impl From<&StateTransitionFactory> for StateTransitionFactoryWasm {
    fn from(factory: &StateTransitionFactory) -> Self {
        Self(factory.to_owned())
    }
}
static mut LOGGER_INITIALIZED: bool = false;

#[wasm_bindgen(js_class=DashPlatformProtocol)]
impl DashPlatformProtocolWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        entropy_generator: ExternalEntropyGenerator,
        maybe_protocol_version: Option<u32>,
    ) -> Result<DashPlatformProtocolWasm, JsValue> {
        // Initialize logger only once to avoid repeating warnings
        // "attempted to set a logger after the logging system was already initialized"
        // Usage of unsafe is fine here as we are in a single-threaded JS environment
        unsafe {
            if !LOGGER_INITIALIZED {
                LOGGER_INITIALIZED = true;
                wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
            }
        }

        // let bls = BlsAdapter(bls_adapter);
        let protocol_version = maybe_protocol_version.unwrap_or(LATEST_VERSION);
        let dpp = DashPlatformProtocol::new(protocol_version);
        Ok(DashPlatformProtocolWasm(dpp))
    }

    // #[wasm_bindgen(getter = dataContract)]
    // pub fn data_contract(&self) -> DataContractFacadeWasm {
    //     self.data_contract.clone()
    // }
    //
    // #[wasm_bindgen(getter=document)]
    // pub fn document(&self) -> DocumentFacadeWasm {
    //     self.document.clone()
    // }
    //
    #[wasm_bindgen(getter = identity)]
    pub fn identity(&self) -> IdentityFacadeWasm {
        self.0.identities().into()
    }

    #[wasm_bindgen(getter = stateTransition)]
    pub fn state_transition(&self) -> StateTransitionFactoryWasm {
        self.0.state_transition().into()
    }
    //
    // #[wasm_bindgen(getter = protocolVersion)]
    // pub fn protocol_version(&self) -> u32 {
    //     self.protocol_version
    // }
    //
    // #[wasm_bindgen(js_name = getProtocolVersion)]
    // pub fn get_protocol_version(&self) -> u32 {
    //     self.protocol_version()
    // }
    //
    // #[wasm_bindgen(js_name = setProtocolVersion)]
    // pub fn set_protocol_version(&mut self, protocol_version: u32) -> Result<(), JsValue> {
    //     self.init(
    //         protocol_version,
    //         self.state_repository.clone(),
    //         self.bls.clone(),
    //         self.entropy_generator.clone(),
    //     )
    // }
    //
    // #[wasm_bindgen(js_name = setStateRepository)]
    // pub fn set_state_repository(
    //     &mut self,
    //     state_repository: ExternalStateRepositoryLike,
    // ) -> Result<(), JsValue> {
    //     self.init(
    //         self.protocol_version,
    //         state_repository,
    //         self.bls.clone(),
    //         self.entropy_generator.clone(),
    //     )
    // }
    //
    // #[wasm_bindgen(js_name = getStateRepository)]
    // pub fn get_state_repository(&self) -> ExternalStateRepositoryLike {
    //     self.state_repository.clone()
    // }
    //
    // fn init(
    //     &mut self,
    //     protocol_version: u32,
    //     state_repository: ExternalStateRepositoryLike,
    //     bls_adapter: BlsAdapter,
    //     entropy_generator: ExternalEntropyGenerator,
    // ) -> Result<(), JsValue> {
    //     let (identity_facade, document_facade, data_contract_facade, state_transition_facade) =
    //         create_facades(
    //             self.public_keys_validator.clone(),
    //             protocol_version,
    //             state_repository.clone(),
    //             bls_adapter,
    //             entropy_generator,
    //         )?;
    //
    //     self.protocol_version = protocol_version;
    //     self.identity = identity_facade;
    //     self.document = document_facade;
    //     self.data_contract = data_contract_facade;
    //     self.state_transition = state_transition_facade;
    //     self.state_repository = state_repository;
    //
    //     Ok(())
    // }
}

// fn create_facades(
//     public_keys_validator: Arc<PublicKeysValidator<BlsAdapter>>,
//     protocol_version: u32,
//     state_repository: ExternalStateRepositoryLike,
//     bls_adapter: BlsAdapter,
//     entropy_generator: ExternalEntropyGenerator,
// ) -> Result<
//     (
//         IdentityFacadeWasm,
//         DocumentFacadeWasm,
//         DataContractFacadeWasm,
//         StateTransitionFacadeWasm,
//     ),
//     JsValue,
// > {
//     let wrapped_state_repository =
//         ExternalStateRepositoryLikeWrapper::new(state_repository.clone());
//     let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
//         protocol_version,
//         LATEST_VERSION,
//         COMPATIBILITY_MAP.clone(),
//     ));
//
//     let identity_facade =
//         IdentityFacadeWasm::new(protocol_version_validator.clone(), public_keys_validator);
//
//     let document_facade = init_document_facade(
//         protocol_version,
//         protocol_version_validator.clone(),
//         wrapped_state_repository,
//         entropy_generator.clone(),
//     );
//
//     let data_contract_facade = DataContractFacadeWasm::new(
//         protocol_version,
//         protocol_version_validator.clone(),
//         entropy_generator,
//     );
//
//     let state_transition_facade =
//         StateTransitionFacadeWasm::new(state_repository, bls_adapter, protocol_version_validator)?;
//
//     Ok((
//         identity_facade,
//         document_facade,
//         data_contract_facade,
//         state_transition_facade,
//     ))
// }
//
// fn init_document_facade(
//     protocol_version: u32,
//     protocol_version_validator: Arc<ProtocolVersionValidator>,
//     state_repository: ExternalStateRepositoryLikeWrapper,
//     entropy_generator: ExternalEntropyGenerator,
// ) -> DocumentFacadeWasm {
//     let document_validator = DocumentValidatorWasm::new_with_arc(protocol_version_validator);
//
//     let document_factory = DocumentFactoryWASM::new_with_state_repository_wrapper(
//         protocol_version,
//         document_validator.clone(),
//         entropy_generator,
//         state_repository.clone(),
//     );
//
//     let data_contract_fetcher_and_validator =
//         DataContractFetcherAndValidatorWasm::new_with_state_repository_wrapper(state_repository);
//
//     DocumentFacadeWasm::new(
//         document_validator,
//         document_factory,
//         data_contract_fetcher_and_validator,
//     )
// }
