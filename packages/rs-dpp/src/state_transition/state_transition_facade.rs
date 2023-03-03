use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use crate::{BlsModule, ProtocolError};
use crate::data_contract::state_transition::data_contract_create_transition::validation::state::validate_data_contract_create_transition_basic::DataContractCreateTransitionBasicValidator;
use crate::data_contract::state_transition::data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator;
use crate::document::validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProofValidator, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator, InstantAssetLockProofStructureValidator};
use crate::identity::state_transition::identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator;
use crate::identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator;
use crate::identity::state_transition::identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator;
use crate::identity::state_transition::identity_update_transition::validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic;
use crate::identity::state_transition::validate_public_key_signatures::PublicKeysSignaturesValidator;
use crate::identity::validation::PublicKeysValidator;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::{StateTransition, StateTransitionFactory};
use crate::state_transition::validation::validate_state_transition_basic::StateTransitionBasicValidator;
use crate::state_transition::validation::validate_state_transition_by_type::StateTransitionByTypeValidator;
use crate::version::ProtocolVersionValidator;

#[derive(Clone)]
pub struct StateTransitionFacade<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule + Clone,
{
    state_repository: Arc<SR>,
    basic_validator:
        Arc<StateTransitionBasicValidator<SR, StateTransitionByTypeValidator<SR, BLS>>>,
    // factory: StateTransitionFactory,
    // state_transition_validator: StateTransitionValidator,
}

impl<SR, BLS> StateTransitionFacade<SR, BLS>
where
    SR: StateRepositoryLike,
    BLS: BlsModule + Clone,
{
    pub fn new(state_repository: Arc<SR>, adapter: BLS) -> Result<Self, ProtocolError> {
        let protocol_version_validator = Arc::new(ProtocolVersionValidator::default());

        let pk_validator = Arc::new(PublicKeysValidator::new(adapter.clone()).map_err(|_| {
            ProtocolError::Generic(String::from("Unable to initialize PublicKeysValidator"))
        })?);
        let pk_sig_validator = Arc::new(PublicKeysSignaturesValidator::new(adapter.clone()));

        let asset_lock_tx_validator =
            Arc::new(AssetLockTransactionValidator::new(state_repository.clone()));

        let asset_lock_validator = Arc::new(AssetLockProofValidator::new(
            InstantAssetLockProofStructureValidator::new(
                state_repository.clone(),
                asset_lock_tx_validator.clone(),
            )
            .map_err(|_| {
                ProtocolError::Generic(String::from(
                    "Unable to initialize InstantAssetLockProofStructureValidator",
                ))
            })?,
            ChainAssetLockProofStructureValidator::new(
                state_repository.clone(),
                asset_lock_tx_validator.clone(),
            )
            .map_err(|_| {
                ProtocolError::Generic(String::from(
                    "Unable to initialize ChainAssetLockProofStructureValidator",
                ))
            })?,
        ));

        let state_transition_basic_validator = StateTransitionBasicValidator::new(
            state_repository.clone(),
            StateTransitionByTypeValidator::new(
                DataContractCreateTransitionBasicValidator::new(
                    protocol_version_validator.clone(),
                )?,
                DataContractUpdateTransitionBasicValidator::new(
                    state_repository.clone(),
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
                    state_repository.clone(),
                    protocol_version_validator.clone(),
                ),
            ),
        );

        Ok(Self {
            state_repository,
            basic_validator: Arc::new(state_transition_basic_validator),
        })
    }

    pub fn validate_basic(&self, state_transition: &StateTransition) {
        // self.state_transition_validator.validate(state_transition)
    }
}
