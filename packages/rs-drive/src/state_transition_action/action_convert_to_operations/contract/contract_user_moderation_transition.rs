use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::contract::contract_user_moderation::v0::ContractUserModerationTransitionActionV0;
use crate::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use crate::util::batch::DriveOperation::{ContractModerationOperation, IdentityOperation};
use crate::util::batch::{ContractModerationOperationType, DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ContractUserModerationTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .contract_user_moderation_transition
        {
            0 => {
                let ContractUserModerationTransitionAction::V0(
                    ContractUserModerationTransitionActionV0 {
                        moderator_id,
                        data_contract_id: contract_id,
                        identity_contract_nonce,
                        action,
                        current_status,
                        ..
                    },
                ) = self;

                let mut operations = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: moderator_id.to_buffer(),
                        contract_id: contract_id.to_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                match action {
                    ContractUserModerationAction::Ban { identity_id } => {
                        // A ban supersedes a suspension, lapsed or not.
                        if current_status.suspended_until.is_some() {
                            operations.push(ContractModerationOperation(
                                ContractModerationOperationType::RemoveSuspension {
                                    contract_id,
                                    identity_id,
                                },
                            ));
                        }
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddBan {
                                contract_id,
                                identity_id,
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Unban { identity_id } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::RemoveBan {
                                contract_id,
                                identity_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Suspend { identity_id, until } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::AddSuspension {
                                contract_id,
                                identity_id,
                                until,
                                replaces_existing: current_status.suspended_until.is_some(),
                                moderator_id,
                            },
                        ));
                    }
                    ContractUserModerationAction::Unsuspend { identity_id } => {
                        operations.push(ContractModerationOperation(
                            ContractModerationOperationType::RemoveSuspension {
                                contract_id,
                                identity_id,
                            },
                        ));
                    }
                }

                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ContractUserModerationTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::moderation::ContractModerationStatus;
    use dpp::platform_value::Identifier;

    fn action(
        action: ContractUserModerationAction,
        current_status: ContractModerationStatus,
    ) -> ContractUserModerationTransitionAction {
        ContractUserModerationTransitionAction::V0(ContractUserModerationTransitionActionV0 {
            moderator_id: Identifier::from([0xAA; 32]),
            data_contract_id: Identifier::from([0xBB; 32]),
            identity_contract_nonce: 4,
            action,
            current_status,
            user_fee_increase: 0,
        })
    }

    #[test]
    fn should_bump_the_contract_nonce_then_edit_the_list() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Ban {
                identity_id: target,
            },
            ContractModerationStatus::default(),
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[0],
            IdentityOperation(IdentityOperationType::UpdateIdentityContractNonce { nonce: 4, .. })
        ));
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddBan { identity_id, .. })
                if *identity_id == target
        ));
    }

    #[test]
    fn should_remove_a_suspension_when_banning_a_suspended_identity() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Ban {
                identity_id: target,
            },
            ContractModerationStatus {
                banned: false,
                suspended_until: Some(10),
            },
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 3);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::RemoveSuspension { .. })
        ));
        assert!(matches!(
            &ops[2],
            ContractModerationOperation(ContractModerationOperationType::AddBan { .. })
        ));
    }

    #[test]
    fn should_replace_an_existing_suspension() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).expect("epoch");
        let target = Identifier::from([0xCC; 32]);

        let ops = action(
            ContractUserModerationAction::Suspend {
                identity_id: target,
                until: 99,
            },
            ContractModerationStatus {
                banned: false,
                suspended_until: Some(10),
            },
        )
        .into_high_level_drive_operations(&epoch, platform_version)
        .expect("operations");
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            &ops[1],
            ContractModerationOperation(ContractModerationOperationType::AddSuspension {
                until: 99,
                replaces_existing: true,
                ..
            })
        ));
    }
}
