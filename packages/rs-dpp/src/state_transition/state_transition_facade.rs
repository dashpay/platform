use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use serde_json::Value;
use crate::{BlsModule, ProtocolError};
use crate::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator;
use crate::data_contract::state_transition::data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator;
use crate::document::validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProofValidator, AssetLockPublicKeyHashFetcher, AssetLockTransactionOutputFetcher, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator, InstantAssetLockProofStructureValidator};
use crate::identity::state_transition::identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator;
use crate::identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator;
use crate::identity::state_transition::identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator;
use crate::identity::state_transition::identity_update_transition::validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic;
use crate::identity::state_transition::validate_public_key_signatures::PublicKeysSignaturesValidator;
use crate::identity::validation::PublicKeysValidator;
use crate::prelude::ValidationResult;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::{StateTransition, StateTransitionFactory, StateTransitionIdentitySigned, StateTransitionLike};
use crate::state_transition::fee::operations::{DeleteOperation, Operation};
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::validation::validate_state_transition_basic::StateTransitionBasicValidator;
use crate::state_transition::validation::validate_state_transition_by_type::StateTransitionByTypeValidator;
use crate::state_transition::validation::validate_state_transition_fee::StateTransitionFeeValidator;
use crate::state_transition::validation::validate_state_transition_identity_signature::validate_state_transition_identity_signature;
use crate::state_transition::validation::validate_state_transition_key_signature::StateTransitionKeySignatureValidator;
use crate::state_transition::validation::validate_state_transition_state::StateTransitionStateValidator;
use crate::validation::{AsyncDataValidator, AsyncDataValidatorWithContext, SimpleValidationResult};
use crate::version::ProtocolVersionValidator;

#[derive(Clone)]
pub struct StateTransitionFacade<SR, BLS>
where
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    state_repository: Arc<SR>,
    basic_validator:
        Arc<StateTransitionBasicValidator<SR, StateTransitionByTypeValidator<SR, BLS>>>,
    state_validator: Arc<StateTransitionStateValidator<SR>>,
    key_signature_validator: Arc<StateTransitionKeySignatureValidator<SR>>,
    fee_validator: Arc<StateTransitionFeeValidator<SR>>,
    bls: BLS, // factory: StateTransitionFactory,
              // state_transition_validator: StateTransitionValidator,
}

impl<SR, BLS> StateTransitionFacade<SR, BLS>
where
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    pub fn new(state_repository: SR, adapter: BLS) -> Result<Self, ProtocolError> {
        let wrapped_state_repository = Arc::new(state_repository.clone());

        let state_transition_basic_validator;
        let state_transition_state_validator;
        let state_transition_key_signature_validator;
        let state_transition_fee_validator;

        {
            let protocol_version_validator = Arc::new(ProtocolVersionValidator::default());

            let pk_validator =
                Arc::new(PublicKeysValidator::new(adapter.clone()).map_err(|_| {
                    ProtocolError::Generic(String::from("Unable to initialize PublicKeysValidator"))
                })?);
            let pk_sig_validator = Arc::new(PublicKeysSignaturesValidator::new(adapter.clone()));

            let asset_lock_tx_validator = Arc::new(AssetLockTransactionValidator::new(
                wrapped_state_repository.clone(),
            ));

            let asset_lock_validator = Arc::new(AssetLockProofValidator::new(
                InstantAssetLockProofStructureValidator::new(
                    wrapped_state_repository.clone(),
                    asset_lock_tx_validator.clone(),
                )
                .map_err(|_| {
                    ProtocolError::Generic(String::from(
                        "Unable to initialize InstantAssetLockProofStructureValidator",
                    ))
                })?,
                ChainAssetLockProofStructureValidator::new(
                    wrapped_state_repository.clone(),
                    asset_lock_tx_validator.clone(),
                )
                .map_err(|_| {
                    ProtocolError::Generic(String::from(
                        "Unable to initialize ChainAssetLockProofStructureValidator",
                    ))
                })?,
            ));

            state_transition_basic_validator = StateTransitionBasicValidator::new(
                wrapped_state_repository.clone(),
                StateTransitionByTypeValidator::new(
                    DataContractCreateTransitionBasicValidator::new(
                        protocol_version_validator.clone(),
                    )?,
                    DataContractUpdateTransitionBasicValidator::new(
                        wrapped_state_repository.clone(),
                        protocol_version_validator.clone(),
                    )
                    .map_err(|_| {
                        ProtocolError::Generic(String::from(
                            "Unable to initialize DataContractUpdateTransitionBasicValidator",
                        ))
                    })?,
                    IdentityCreateTransitionBasicValidator::new(
                        protocol_version_validator.deref().clone(),
                        pk_validator.clone(),
                        pk_validator.clone(),
                        asset_lock_validator.clone(),
                        adapter.clone(),
                        pk_sig_validator.clone(),
                    )
                    .map_err(|_| {
                        ProtocolError::Generic(String::from(
                            "Unable to initialize IdentityCreateTransitionBasicValidator",
                        ))
                    })?,
                    ValidateIdentityUpdateTransitionBasic::new(
                        ProtocolVersionValidator::default(),
                        pk_validator.clone(),
                        pk_sig_validator.clone(),
                    )?,
                    IdentityTopUpTransitionBasicValidator::new(
                        ProtocolVersionValidator::default(),
                        asset_lock_validator.clone(),
                    )
                    .map_err(|_| {
                        ProtocolError::Generic(String::from(
                            "Unable to initialize IdentityTopUpTransitionBasicValidator",
                        ))
                    })?,
                    IdentityCreditWithdrawalTransitionBasicValidator::new(
                        protocol_version_validator.clone(),
                    )
                    .map_err(|_| {
                        ProtocolError::Generic(String::from(
                            "Unable to initialize IdentityCreditWithdrawalTransitionBasicValidator",
                        ))
                    })?,
                    DocumentBatchTransitionBasicValidator::new(
                        wrapped_state_repository.clone(),
                        protocol_version_validator.clone(),
                    ),
                ),
            );
        }

        {
            let asset_lock_transaction_output_fetcher =
                AssetLockTransactionOutputFetcher::new(wrapped_state_repository.clone());

            let asset_public_key_hash_fetcher = AssetLockPublicKeyHashFetcher::new(
                wrapped_state_repository.clone(),
                asset_lock_transaction_output_fetcher,
            );

            state_transition_key_signature_validator = StateTransitionKeySignatureValidator::new(
                wrapped_state_repository.clone(),
                asset_public_key_hash_fetcher,
            );
        }

        state_transition_fee_validator =
            StateTransitionFeeValidator::new(wrapped_state_repository.clone());

        state_transition_state_validator =
            StateTransitionStateValidator::new(state_repository.clone());

        Ok(Self {
            state_repository: wrapped_state_repository,
            basic_validator: Arc::new(state_transition_basic_validator),
            key_signature_validator: Arc::new(state_transition_key_signature_validator),
            fee_validator: Arc::new(state_transition_fee_validator),
            state_validator: Arc::new(state_transition_state_validator),
            bls: adapter,
        })
    }

    pub async fn validate_basic(
        &self,
        state_transition: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        self.basic_validator
            .validate(state_transition, execution_context)
            .await
    }

    pub async fn validate_signature(
        &self,
        mut state_transition: StateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        // TODO: can we avoid duplicated code here?
        return match state_transition {
            StateTransition::DataContractCreate(ref mut st) => {
                validate_state_transition_identity_signature(
                    self.state_repository.clone(),
                    st,
                    &self.bls,
                )
                .await
            }
            StateTransition::DataContractUpdate(ref mut st) => {
                validate_state_transition_identity_signature(
                    self.state_repository.clone(),
                    st,
                    &self.bls,
                )
                .await
            }
            StateTransition::DocumentsBatch(ref mut st) => {
                validate_state_transition_identity_signature(
                    self.state_repository.clone(),
                    st,
                    &self.bls,
                )
                .await
            }
            StateTransition::IdentityCreditWithdrawal(ref mut st) => {
                validate_state_transition_identity_signature(
                    self.state_repository.clone(),
                    st,
                    &self.bls,
                )
                .await
            }
            StateTransition::IdentityUpdate(ref mut st) => {
                validate_state_transition_identity_signature(
                    self.state_repository.clone(),
                    st,
                    &self.bls,
                )
                .await
            }
            _ => {
                self.key_signature_validator
                    .validate(&state_transition)
                    .await
            }
        };
    }

    pub async fn validate_fee(
        &self,
        state_transition: &StateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        self.fee_validator.validate(state_transition).await
    }

    pub async fn validate_state(
        &self,
        state_transition: &StateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        self.state_validator.validate(state_transition).await
    }

    // pub async fn validate() -> Result<SimpleValidationResult, ProtocolError> {}
}
