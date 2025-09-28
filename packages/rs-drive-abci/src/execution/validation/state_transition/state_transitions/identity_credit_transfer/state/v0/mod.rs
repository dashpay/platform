use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::signature::IdentityNotFoundError;

use dpp::consensus::state::identity::IdentityInsufficientBalanceError;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use drive::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;

use dpp::version::PlatformVersion;
use drive::drive::subscriptions::{DriveSubscriptionFilter, HitFiltersType};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_transfer) trait IdentityCreditTransferStateTransitionStateValidationV0 {
    fn validate_state_v0<'a, C>(
        &self,
        platform: &PlatformRef<C>,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;

    fn transform_into_action_v0<'a, C>(
        &self,
        platform: &PlatformRef<C>,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;
}

impl IdentityCreditTransferStateTransitionStateValidationV0 for IdentityCreditTransferTransition {
    fn validate_state_v0<'a, C>(
        &self,
        platform: &PlatformRef<C>,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let maybe_existing_identity_balance = platform.drive.fetch_identity_balance(
            self.identity_id().to_buffer(),
            tx,
            platform_version,
        )?;

        let Some(existing_identity_balance) = maybe_existing_identity_balance else {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.identity_id()).into(),
            ));
        };

        if existing_identity_balance < self.amount() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    existing_identity_balance,
                    self.amount(),
                )
                .into(),
            ));
        }

        let maybe_existing_recipient = platform.drive.fetch_identity_balance(
            self.recipient_id().to_buffer(),
            tx,
            platform_version,
        )?;

        if maybe_existing_recipient.is_none() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.recipient_id()).into(),
            ));
        }

        self.transform_into_action_v0(passing_filters_for_transition)
    }

    fn transform_into_action_v0<'a, C>(
        &self,
        platform: &PlatformRef<C>,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let filters_hit = if passing_filters_for_transition.is_empty() {
            HitFiltersType::NoFilterHit
        } else {
            platform.drive.
            HitFiltersType::DidHitFilters {
                original_grovedb_proof: vec![],
                filters_hit: passing_filters_for_transition,
            }
        };
        Ok(ConsensusValidationResult::new_with_data(
            TransformToStateTransitionActionResult {
                action: IdentityCreditTransferTransitionAction::from(self).into(),
                filters_hit: HitFiltersType::NoFilterHit,
            }
        ))
    }
}
