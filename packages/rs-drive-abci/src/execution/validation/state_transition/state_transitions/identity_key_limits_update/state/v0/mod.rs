use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::validate_identity_public_key_ids_exist_in_state;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::identity_public_key_already_expired_error::IdentityPublicKeyAlreadyExpiredError;
use dpp::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use dpp::consensus::state::identity::identity_public_key_limit_not_raised_error::IdentityPublicKeyLimitNotRaisedError;
use dpp::consensus::state::identity::identity_public_key_limit_not_set_error::{
    IdentityPublicKeyLimitNotSetError, KeyLimit,
};
use dpp::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;
use dpp::consensus::ConsensusError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_key_limits_update::IdentityKeyLimitsUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_key_limits_update) trait IdentityKeyLimitsUpdateStateTransitionStateValidationV0
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityKeyLimitsUpdateStateTransitionStateValidationV0 for IdentityKeyLimitsUpdateTransition {
    /// Reads the key and checks that the update only loosens its limits: the key exists and is
    /// enabled, it has every limit the transition raises, each new value is greater than the
    /// current one, and the key is not expired once the update is applied. Every refusal is paid
    /// for by bumping the identity nonce.
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Priced like the retrieval of one more key of the identity: the shared helper reads
        // the key without billing it.
        execution_context.add_operation(ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::one_key(),
        ));
        let key_result = validate_identity_public_key_ids_exist_in_state(
            self.identity_id(),
            &[self.key_id()],
            platform.drive,
            execution_context,
            tx,
            platform_version,
        )?;

        let bump_action = || {
            StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_identity_key_limits_update_transition(self),
            )
        };

        if !key_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                key_result.errors,
            ));
        }

        let keys: Vec<IdentityPublicKey> = key_result.into_data()?;
        let Some(key) = keys.first() else {
            // The helper reports a missing key as an error above, so a valid empty answer is
            // treated the same way rather than trusted.
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                vec![MissingIdentityPublicKeyIdsError::new(vec![self.key_id()]).into()],
            ));
        };

        if let Some(error) = refusal_for_key(self, key, block_info) {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                vec![error],
            ));
        }

        self.transform_into_action_v0()
    }

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        validation_result.set_data(IdentityKeyLimitsUpdateTransitionAction::from(self).into());
        Ok(validation_result)
    }
}

/// The first rule the stored key breaks for this update, if any.
fn refusal_for_key(
    transition: &IdentityKeyLimitsUpdateTransition,
    key: &IdentityPublicKey,
    block_info: &BlockInfo,
) -> Option<ConsensusError> {
    {
        let key_id = transition.key_id();

        if key.disabled_at().is_some() {
            return Some(IdentityPublicKeyIsDisabledError::new(key_id).into());
        }

        if let Some(total_budget) = transition.total_budget() {
            let Some(current) = key.total_budget() else {
                return Some(
                    IdentityPublicKeyLimitNotSetError::new(key_id, KeyLimit::Budget).into(),
                );
            };
            if total_budget <= current {
                return Some(
                    IdentityPublicKeyLimitNotRaisedError::new(
                        key_id,
                        KeyLimit::Budget,
                        current,
                        total_budget,
                    )
                    .into(),
                );
            }
        }

        if let Some(expires_at) = transition.expires_at() {
            let Some(current) = key.expires_at() else {
                return Some(
                    IdentityPublicKeyLimitNotSetError::new(key_id, KeyLimit::Expiry).into(),
                );
            };
            if expires_at <= current {
                return Some(
                    IdentityPublicKeyLimitNotRaisedError::new(
                        key_id,
                        KeyLimit::Expiry,
                        current,
                        expires_at,
                    )
                    .into(),
                );
            }
        }

        // An expired key may be revived by an extension, but not topped up while it stays
        // expired: after the update the key must be usable.
        if let Some(expires_at) = transition.expires_at().or(key.expires_at()) {
            if block_info.time_ms >= expires_at {
                return Some(
                    IdentityPublicKeyAlreadyExpiredError::new(
                        key_id,
                        expires_at,
                        block_info.time_ms,
                    )
                    .into(),
                );
            }
        }

        None
    }
}
