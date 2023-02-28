use std::{ops::Deref, sync::Arc};

use serde_json::Value as JsonValue;

use dpp::{
    state_transition::{
        validation::{
            validate_state_transition_basic::StateTransitionBasicValidator,
            validate_state_transition_by_type::StateTransitionByTypeValidator,
        },
        StateTransitionFactory, StateTransitionFactoryOptions, StateTransition, errors::StateTransitionError,
    },
    version::ProtocolVersionValidator, data_contract::state_transition::{data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator, data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator, self}, identity::{state_transition::{identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator, validate_public_key_signatures::{PublicKeysSignaturesValidator, TPublicKeysSignaturesValidator}, asset_lock_proof::{AssetLockProofValidator, ChainAssetLockProofStructureValidator, InstantAssetLockProofStructureValidator, AssetLockTransactionValidator}, identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator, identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator, identity_update_transition::validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic}, validation::PublicKeysValidator}, document::validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator,
};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::{
    bls_adapter::{BlsAdapter, JsBlsAdapter},
    errors::{from_dpp_err, from_dpp_init_error},
    identity_facade::FromObjectOptions,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    state_transition::errors::invalid_state_transition_error::InvalidStateTransitionError,
    utils, with_js_error, StateTransitionWasm,
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

#[wasm_bindgen(js_class = StateTransitionFactory)]
impl StateTransitionFactoryWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
        bls_adapter: JsBlsAdapter,
    ) -> Result<StateTransitionFactoryWasm, JsValue> {
        let state_repository_wrapper = ExternalStateRepositoryLikeWrapper::new(state_repository);
        let protocol_version_validator = Arc::new(ProtocolVersionValidator::default());

        let pk_validator =
            Arc::new(PublicKeysValidator::new(bls_adapter.into()).map_err(from_dpp_init_error)?);
        let pk_sig_validator = Arc::new(PublicKeysSignaturesValidator::new(bls_adapter.into()));

        let asset_lock_tx_validator = Arc::new(AssetLockTransactionValidator::new(
            state_repository_wrapper.into(),
        ));

        let asset_lock_validator = Arc::new(AssetLockProofValidator::new(
            InstantAssetLockProofStructureValidator::new(
                state_repository_wrapper.into(),
                asset_lock_tx_validator,
            )
            .map_err(from_dpp_init_error)?,
            ChainAssetLockProofStructureValidator::new(
                state_repository_wrapper.into(),
                asset_lock_tx_validator,
            )
            .map_err(from_dpp_init_error)?,
        ));

        let factory = StateTransitionFactory::new(
            state_repository_wrapper.into(),
            StateTransitionBasicValidator::new(
                state_repository_wrapper.into(),
                StateTransitionByTypeValidator::new(
                    DataContractCreateTransitionBasicValidator::new(protocol_version_validator)
                        .map_err(from_dpp_err)?,
                    DataContractUpdateTransitionBasicValidator::new(
                        state_repository_wrapper.into(),
                        protocol_version_validator,
                    )
                    .map_err(from_dpp_init_error)?,
                    IdentityCreateTransitionBasicValidator::new(
                        protocol_version_validator.deref().clone(),
                        pk_validator,
                        pk_validator,
                        asset_lock_validator,
                        bls_adapter.into(),
                        pk_sig_validator,
                    )
                    .map_err(from_dpp_init_error)?,
                    ValidateIdentityUpdateTransitionBasic::new(
                        protocol_version_validator,
                        pk_validator,
                        pk_sig_validator,
                    )
                    .map_err(from_dpp_err)?,
                    IdentityTopUpTransitionBasicValidator::new(
                        protocol_version_validator,
                        asset_lock_validator,
                    )
                    .map_err(from_dpp_init_error)?,
                    IdentityCreditWithdrawalTransitionBasicValidator::new(
                        protocol_version_validator,
                    )
                    .map_err(from_dpp_init_error)?,
                    DocumentBatchTransitionBasicValidator::new(
                        state_repository_wrapper.into(),
                        protocol_version_validator,
                    ),
                ),
            ),
        );

        Ok(factory.into())
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        state_transition_object: JsValue,
        options: JsValue,
    ) -> Result<StateTransitionWasm, JsValue> {
        let options: FromObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let state_transition_json_string = utils::stringify(&state_transition_object)?;
        let raw_state_transition: JsonValue =
            serde_json::from_str(&state_transition_json_string).map_err(|e| e.to_string())?;

        let result = self
            .0
            .create_from_object(
                raw_state_transition,
                Some(StateTransitionFactoryOptions {
                    skip_validation: options.skip_validation.unwrap_or(false),
                }),
            )
            .await;

        match result {
            Ok(state_transition) => Ok(StateTransitionWasm::from(state_transition)),
            Err(dpp::ProtocolError::StateTransitionError(e)) => match e {
                StateTransitionError::InvalidStateTransitionError {
                    errors,
                    raw_state_transition,
                } => Err(InvalidStateTransitionError::new(errors, raw_state_transition).into()),
            },
            Err(other) => Err(from_dpp_err(other)),
        }
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: JsValue,
    ) -> Result<StateTransitionWasm, JsValue> {
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

        match result {
            Ok(state_transition) => Ok(StateTransitionWasm::from(state_transition)),
            Err(dpp::ProtocolError::StateTransitionError(e)) => match e {
                StateTransitionError::InvalidStateTransitionError {
                    errors,
                    raw_state_transition,
                } => Err(InvalidStateTransitionError::new(errors, raw_state_transition).into()),
            },
            Err(other) => Err(from_dpp_err(other)),
        }
    }
}
