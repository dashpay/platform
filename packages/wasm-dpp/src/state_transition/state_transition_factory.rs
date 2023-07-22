use std::{ops::Deref, sync::Arc};

use dpp::{state_transition::{
    state_transition_validation::{
        validate_state_transition_basic::StateTransitionBasicValidator,
        validate_state_transition_by_type::StateTransitionByTypeValidator,
    },
    StateTransitionFactory, StateTransitionFactoryOptions, StateTransition, errors::StateTransitionError,
}, version::ProtocolVersionValidator, data_contract::state_transition::{data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator, data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator}, identity::{state_transition::{identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator, validate_public_key_signatures::{PublicKeysSignaturesValidator}, asset_lock_proof::{AssetLockProofValidator, ChainAssetLockProofStructureValidator, InstantAssetLockProofStructureValidator, AssetLockTransactionValidator}, identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator, identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator, identity_update_transition::validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic}, validation::PublicKeysValidator}, document::validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator, ProtocolError};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use dpp::identity::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IdentityCreditTransferTransitionBasicValidator;
use dpp::platform_value::Value;

use crate::utils::{ToSerdeJSONExt, WithJsError};
use crate::{
    bls_adapter::{BlsAdapter, JsBlsAdapter},
    errors::{from_dpp_err, from_dpp_init_error},
    identity_facade::FromObjectOptions,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    state_transition::errors::invalid_state_transition_error::InvalidStateTransitionErrorWasm,
    with_js_error, DataContractCreateTransitionWasm, DataContractUpdateTransitionWasm,
    DocumentsBatchTransitionWasm, IdentityCreateTransitionWasm,
    IdentityCreditTransferTransitionWasm, IdentityTopUpTransitionWasm,
    IdentityUpdateTransitionWasm,
};

#[wasm_bindgen(js_name = StateTransitionFactory)]
pub struct StateTransitionFactoryWasm(
    StateTransitionFactory<ExternalStateRepositoryLikeWrapper, BlsAdapter>,
);

impl From<StateTransitionFactory<ExternalStateRepositoryLikeWrapper, BlsAdapter>>
    for StateTransitionFactoryWasm
{
    fn from(
        factory: StateTransitionFactory<ExternalStateRepositoryLikeWrapper, BlsAdapter>,
    ) -> Self {
        Self(factory)
    }
}

impl StateTransitionFactoryWasm {
    pub fn state_transition_wasm_from_factory_result(
        result: Result<StateTransition, ProtocolError>,
    ) -> Result<JsValue, JsValue> {
        match result {
            Ok(state_transition) => match state_transition {
                StateTransition::DataContractCreate(st) => {
                    Ok(DataContractCreateTransitionWasm::from(st).into())
                }
                StateTransition::DataContractUpdate(st) => {
                    Ok(DataContractUpdateTransitionWasm::from(st).into())
                }
                StateTransition::IdentityCreate(st) => {
                    Ok(IdentityCreateTransitionWasm::from(st).into())
                }
                StateTransition::IdentityUpdate(st) => {
                    Ok(IdentityUpdateTransitionWasm::from(st).into())
                }
                StateTransition::IdentityTopUp(st) => {
                    Ok(IdentityTopUpTransitionWasm::from(st).into())
                }
                StateTransition::IdentityCreditTransfer(st) => {
                    Ok(IdentityCreditTransferTransitionWasm::from(st).into())
                }
                StateTransition::DocumentsBatch(st) => {
                    Ok(DocumentsBatchTransitionWasm::from(st).into())
                }
                _ => Err("Unsupported state transition type".into()),
            },
            Err(dpp::ProtocolError::StateTransitionError(e)) => match e {
                StateTransitionError::InvalidStateTransitionError {
                    errors,
                    raw_state_transition,
                } => Err(InvalidStateTransitionErrorWasm::new(
                    errors,
                    serde_wasm_bindgen::to_value(&raw_state_transition)?,
                )
                .into()),
            },
            Err(other) => Err(from_dpp_err(other)),
        }
    }
}

#[wasm_bindgen(js_class = StateTransitionFactory)]
impl StateTransitionFactoryWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
        bls_adapter: JsBlsAdapter,
    ) -> Result<StateTransitionFactoryWasm, JsValue> {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));
        let protocol_version_validator = Arc::new(ProtocolVersionValidator::default());

        let adapter = BlsAdapter(bls_adapter);

        let pk_validator =
            Arc::new(PublicKeysValidator::new(adapter.clone()).map_err(from_dpp_init_error)?);
        let pk_sig_validator = Arc::new(PublicKeysSignaturesValidator::new(adapter.clone()));

        let asset_lock_tx_validator = Arc::new(AssetLockTransactionValidator::new(
            state_repository_wrapper.clone(),
        ));

        let asset_lock_validator = Arc::new(AssetLockProofValidator::new(
            InstantAssetLockProofStructureValidator::new(
                state_repository_wrapper.clone(),
                asset_lock_tx_validator.clone(),
            )
            .map_err(from_dpp_init_error)?,
            ChainAssetLockProofStructureValidator::new(
                state_repository_wrapper.clone(),
                asset_lock_tx_validator,
            )
            .map_err(from_dpp_init_error)?,
        ));

        let state_transition_basic_validator = StateTransitionBasicValidator::new(
            state_repository_wrapper.clone(),
            StateTransitionByTypeValidator::new(
                DataContractCreateTransitionBasicValidator::new(protocol_version_validator.clone())
                    .with_js_error()?,
                DataContractUpdateTransitionBasicValidator::new(
                    state_repository_wrapper.clone(),
                    protocol_version_validator.clone(),
                )
                .map_err(from_dpp_init_error)?,
                IdentityCreateTransitionBasicValidator::new(
                    protocol_version_validator.deref().clone(),
                    pk_validator.clone(),
                    pk_validator.clone(),
                    asset_lock_validator.clone(),
                    adapter,
                    pk_sig_validator.clone(),
                )
                .map_err(from_dpp_init_error)?,
                ValidateIdentityUpdateTransitionBasic::new(
                    ProtocolVersionValidator::default(),
                    pk_validator,
                    pk_sig_validator,
                )
                .with_js_error()?,
                IdentityTopUpTransitionBasicValidator::new(
                    ProtocolVersionValidator::default(),
                    asset_lock_validator,
                )
                .map_err(from_dpp_init_error)?,
                IdentityCreditWithdrawalTransitionBasicValidator::new(
                    protocol_version_validator.clone(),
                )
                .map_err(from_dpp_init_error)?,
                DocumentBatchTransitionBasicValidator::new(
                    state_repository_wrapper.clone(),
                    protocol_version_validator.clone(),
                ),
                IdentityCreditTransferTransitionBasicValidator::new(
                    ProtocolVersionValidator::default(),
                )
                .map_err(from_dpp_init_error)?,
            ),
        );

        let factory = StateTransitionFactory::new(
            state_repository_wrapper,
            Arc::new(state_transition_basic_validator),
        );

        Ok(factory.into())
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        state_transition_object: JsValue,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let raw_state_transition: Value = state_transition_object.with_serde_to_platform_value()?;

        let result = self
            .0
            .create_from_object(
                raw_state_transition,
                Some(StateTransitionFactoryOptions {
                    skip_validation: options.skip_validation.unwrap_or(false),
                }),
            )
            .await;

        Self::state_transition_wasm_from_factory_result(result)
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let result = self
            .0
            .create_from_buffer(
                &buffer,
                Some(StateTransitionFactoryOptions {
                    skip_validation: options.skip_validation.unwrap_or(false),
                }),
            )
            .await;

        Self::state_transition_wasm_from_factory_result(result)
    }
}
