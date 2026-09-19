use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::state::contract_moderation::{
    ContractModerationNotEnabledError, ContractModerationTargetNotAllowedError,
    ContractModerationTargetNotFoundError, ContractSuspensionNotInFutureError,
    ContractUserAlreadyBannedError, ContractUserNotBannedError, ContractUserNotSuspendedError,
    IdentityNotContractModeratorError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationList, ContractModerationStatus,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransitionOwned;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::contract_user_moderation) trait ContractUserModerationStateTransitionStateValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ContractUserModerationStateTransitionStateValidationV0 for ContractUserModerationTransition {
    /// Reads the contract and the target's status and checks the moderation: the contract
    /// keeps the list the action edits, the signer is its owner or one of its moderators, the
    /// target is neither and exists, and the action fits the target's status. Every refusal
    /// after the contract is found is paid for by bumping the signer's contract nonce.
    ///
    /// The action carries the target's status as read here, so Drive edits the lists without
    /// reading them again, and the mempool, which transforms without a state validation stage,
    /// refuses with the same consensus codes as a block.
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let contract_id = self.data_contract_id();
        let moderator_id = self.owner_id();
        let action = self.action();
        let target_id = action.identity_id();

        let Some(contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                tx,
                platform_version,
            )?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                DataContractNotPresentError::new(contract_id).into(),
            ));
        };
        if let Some(fee) = contract_fetch_info.fee.clone() {
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        }

        let bump_action = || {
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                    self,
                ),
            )
        };
        let refuse = |error: ConsensusError| {
            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                vec![error],
            ))
        };

        let contract = &contract_fetch_info.contract;
        let list = list_of(&action);

        let Some(moderation) = contract.config().moderation() else {
            return refuse(ContractModerationNotEnabledError::new(contract_id, list).into());
        };
        if !moderation.keeps(list) {
            return refuse(ContractModerationNotEnabledError::new(contract_id, list).into());
        }

        let owner_id = contract.owner_id();
        if !moderation.may_moderate(&owner_id, &moderator_id) {
            return refuse(
                IdentityNotContractModeratorError::new(contract_id, moderator_id).into(),
            );
        }
        // The owner and the moderators cannot be put on a list. They can be taken off one: a
        // contract update may name as moderator an identity that already carries an entry, and
        // without the removal that entry could only be lifted by demoting the moderator first.
        let adds_an_entry = matches!(
            action,
            ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Suspend { .. }
        );
        if adds_an_entry && moderation.is_owner_or_moderator(&owner_id, &target_id) {
            return refuse(
                ContractModerationTargetNotAllowedError::new(contract_id, target_id).into(),
            );
        }

        if !validate_identity_exists(
            platform.drive,
            &target_id,
            execution_context,
            tx,
            platform_version,
        )? {
            return refuse(
                ContractModerationTargetNotFoundError::new(contract_id, target_id).into(),
            );
        }

        let lists = lists_to_read(moderation, &action);
        let (fee, status) = platform.drive.fetch_contract_moderation_status_with_fee(
            contract_id,
            target_id,
            &lists,
            &block_info.epoch,
            tx,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        if let Some(error) = refusal_for_status(&action, &status, contract_id, block_info) {
            return refuse(error);
        }

        Ok(ConsensusValidationResult::new_with_data(
            ContractUserModerationTransitionAction::from_borrowed_transition_with_status(
                self, status,
            )
            .into(),
        ))
    }
}

/// The list the action edits.
fn list_of(action: &ContractUserModerationAction) -> ContractModerationList {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Unban { .. } => {
            ContractModerationList::Banlist
        }
        ContractUserModerationAction::Suspend { .. }
        | ContractUserModerationAction::Unsuspend { .. } => ContractModerationList::Suspensions,
    }
}

/// The lists the action needs to know about: its own, and for a ban or a suspend the other
/// one as well, because a ban removes a suspension and a suspend is refused for a banned
/// identity. Only lists the contract keeps are read.
fn lists_to_read(
    moderation: &ContractModerationConfig,
    action: &ContractUserModerationAction,
) -> Vec<ContractModerationList> {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Suspend { .. } => {
            moderation.lists().collect()
        }
        ContractUserModerationAction::Unban { .. }
        | ContractUserModerationAction::Unsuspend { .. } => vec![list_of(action)],
    }
}

/// The first rule the target's status breaks for this action, if any.
fn refusal_for_status(
    action: &ContractUserModerationAction,
    status: &ContractModerationStatus,
    contract_id: Identifier,
    block_info: &BlockInfo,
) -> Option<ConsensusError> {
    let target_id = action.identity_id();
    match action {
        ContractUserModerationAction::Ban { .. } => status
            .banned
            .then(|| ContractUserAlreadyBannedError::new(contract_id, target_id).into()),
        ContractUserModerationAction::Unban { .. } => {
            (!status.banned).then(|| ContractUserNotBannedError::new(contract_id, target_id).into())
        }
        ContractUserModerationAction::Suspend { until, .. } => {
            if status.banned {
                return Some(ContractUserAlreadyBannedError::new(contract_id, target_id).into());
            }
            (*until <= block_info.time_ms).then(|| {
                ContractSuspensionNotInFutureError::new(
                    contract_id,
                    target_id,
                    *until,
                    block_info.time_ms,
                )
                .into()
            })
        }
        ContractUserModerationAction::Unsuspend { .. } => status
            .suspended_until
            .is_none()
            .then(|| ContractUserNotSuspendedError::new(contract_id, target_id).into()),
    }
}
