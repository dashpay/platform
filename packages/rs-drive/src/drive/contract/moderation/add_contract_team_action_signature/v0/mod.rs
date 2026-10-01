use crate::drive::contract::moderation::types::{
    encode_contract_team_action, ContractTeamActionWrite,
};
use crate::drive::contract::paths::{
    contract_team_action_path, contract_team_action_path_vec, contract_team_action_signers_path,
    contract_team_action_signers_path_vec, contract_team_action_status_path,
    CONTRACT_TEAM_ACTION_INFO_KEY, CONTRACT_TEAM_ACTION_SIGNERS_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{
    BatchDeleteApplyType, BatchInsertTreeApplyType, BatchMoveApplyType,
};
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::MaybeTree;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_team_action_signature_operations_v0(
        &self,
        contract_id: Identifier,
        action_id: Identifier,
        signer_id: Identifier,
        write: &ContractTeamActionWrite,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let estimating = estimated_costs_only_with_layer_info.is_some();
        let closes = match write {
            ContractTeamActionWrite::Propose { closes, .. } => *closes,
            ContractTeamActionWrite::Approve { .. } => false,
            ContractTeamActionWrite::Close { .. } => true,
        };
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_team_action(
                contract_id.to_buffer(),
                action_id.to_buffer(),
                closes,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let drive_version = &platform_version.drive;
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        let contract = contract_id.as_slice();
        let action = action_id.as_slice();
        // Where the action is written: the closed actions when this signature closes it, the
        // active ones otherwise. Written closed, nothing carries flags: the member whose
        // signature closes the action pays for its closed copy for good, as a token group's.
        let (status, flags) = if closes {
            (GroupActionStatus::ActionClosed, None)
        } else {
            (
                GroupActionStatus::ActionActive,
                Some(StorageFlags::new_single_epoch(
                    block_info.epoch.index,
                    Some(signer_id.to_buffer()),
                )),
            )
        };
        let element_flags = StorageFlags::map_to_some_element_flags(flags.as_ref());

        // The action's own tree: created by its proposal, and by the signature that closes it
        // under the closed actions. An action id is taken once (it commits to its proposer's
        // nonce), so an existing tree is a state no transition produces.
        if !matches!(write, ContractTeamActionWrite::Approve { .. }) {
            let tree_apply_type = if estimating {
                BatchInsertTreeApplyType::StatelessBatchInsertTree {
                    in_tree_type: TreeType::NormalTree,
                    tree_type: TreeType::NormalTree,
                    flags_len: 0,
                }
            } else {
                BatchInsertTreeApplyType::StatefulBatchInsertTree
            };
            let inserted = self.batch_insert_empty_tree_if_not_exists(
                PathFixedSizeKeyRef((contract_team_action_status_path(contract, status), action)),
                TreeType::NormalTree,
                None,
                tree_apply_type,
                transaction,
                &mut None,
                &mut batch_operations,
                drive_version,
            )?;
            if !inserted && !estimating {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a team action's proposal or closing must create the action's tree",
                )));
            }
            self.batch_insert_empty_sum_tree(
                contract_team_action_path(contract, status, action),
                DriveKeyInfo::KeyRef(CONTRACT_TEAM_ACTION_SIGNERS_KEY),
                None,
                &mut batch_operations,
                drive_version,
            )?;
        }

        // The signer's approval, one of the sum
        self.batch_insert(
            PathKeyElementInfo::PathKeyElement::<0>((
                contract_team_action_signers_path_vec(contract, status, action),
                signer_id.to_vec(),
                Element::new_sum_item_with_flags(1, element_flags.clone()),
            )),
            &mut batch_operations,
            drive_version,
        )?;

        match write {
            // A proposal writes the action, active or, when it meets the rule alone, closed
            ContractTeamActionWrite::Propose {
                action: proposal, ..
            } => {
                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<6>((
                        contract_team_action_path(contract, status, action),
                        CONTRACT_TEAM_ACTION_INFO_KEY,
                        Element::new_item_with_flags(
                            encode_contract_team_action(proposal),
                            element_flags,
                        ),
                    )),
                    &mut batch_operations,
                    drive_version,
                )?;
            }
            // An approval that stays active deletes the approvals of members who left, which
            // refunds each to whoever its flags name
            ContractTeamActionWrite::Approve { dropped_signers } => {
                for dropped_signer in dropped_signers {
                    self.batch_delete(
                        contract_team_action_signers_path(contract, status, action)
                            .as_slice()
                            .into(),
                        dropped_signer.as_slice(),
                        signer_delete_apply_type(estimating),
                        transaction,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }
            }
            // An approval that closes the action moves it, the earlier approvals that counted and
            // its info, to the closed actions without flags, deletes the approvals of members who
            // left, which refunds each to whoever its flags name, and deletes what is left of it
            // under the active actions. A move is a delete and an insert of the element as it is
            // known here (an approval is a sum item of 1; the info is the action as stored, which
            // encodes back to its bytes), so nothing is read again.
            ContractTeamActionWrite::Close {
                action: stored,
                earlier_signers,
                dropped_signers,
            } => {
                let active = GroupActionStatus::ActionActive;
                let closed = GroupActionStatus::ActionClosed;
                for dropped_signer in dropped_signers {
                    self.batch_delete(
                        contract_team_action_signers_path(contract, active, action)
                            .as_slice()
                            .into(),
                        dropped_signer.as_slice(),
                        signer_delete_apply_type(estimating),
                        transaction,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }
                for earlier_signer in earlier_signers {
                    self.move_known_team_action_element(
                        contract_team_action_signers_path(contract, active, action),
                        earlier_signer.as_slice(),
                        contract_team_action_signers_path_vec(contract, closed, action),
                        Element::new_sum_item(1),
                        TreeType::SumTree,
                        8,
                        estimating,
                        transaction,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }
                let encoded_info = encode_contract_team_action(stored);
                let encoded_info_size = encoded_info.len() as u32;
                self.move_known_team_action_element(
                    contract_team_action_path(contract, active, action),
                    CONTRACT_TEAM_ACTION_INFO_KEY,
                    contract_team_action_path_vec(contract, closed, action),
                    Element::new_item(encoded_info),
                    TreeType::NormalTree,
                    encoded_info_size,
                    estimating,
                    transaction,
                    &mut batch_operations,
                    drive_version,
                )?;

                let delete_apply_type = |tree_type: TreeType| {
                    if estimating {
                        BatchDeleteApplyType::StatelessBatchDelete {
                            in_tree_type: TreeType::NormalTree,
                            estimated_key_size: 32,
                            estimated_value_size: 32,
                        }
                    } else {
                        BatchDeleteApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(tree_type)),
                        }
                    }
                };
                self.batch_delete(
                    contract_team_action_path(contract, active, action)
                        .as_slice()
                        .into(),
                    CONTRACT_TEAM_ACTION_SIGNERS_KEY,
                    delete_apply_type(TreeType::SumTree),
                    transaction,
                    &mut batch_operations,
                    drive_version,
                )?;
                self.batch_delete(
                    contract_team_action_status_path(contract, active)
                        .as_slice()
                        .into(),
                    action,
                    delete_apply_type(TreeType::NormalTree),
                    transaction,
                    &mut batch_operations,
                    drive_version,
                )?;
            }
        }

        Ok(batch_operations)
    }
}

/// How the deletion of one approval is applied: an approval is a sum item in the sum tree of
/// its action's approvals.
fn signer_delete_apply_type(estimating: bool) -> BatchDeleteApplyType {
    if estimating {
        BatchDeleteApplyType::StatelessBatchDelete {
            in_tree_type: TreeType::SumTree,
            estimated_key_size: 32,
            estimated_value_size: 8,
        }
    } else {
        BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        }
    }
}

impl Drive {
    /// Moves `key` from `from_path` to `to_path`, without flags, for a team action that closes:
    /// the element is known (`element`, as stored less its flags), so applying it deletes the
    /// stored one, refunding whoever its flags name, and inserts `element`, reading nothing. An
    /// estimate prices it as GroveDB's average-case move does, the moved value at
    /// `estimated_value_size` bytes in a tree of `in_tree_type`: a sum item estimated by its
    /// serialized size alone would fall short of the fixed cost it is charged.
    #[allow(clippy::too_many_arguments)]
    fn move_known_team_action_element<const N: usize>(
        &self,
        from_path: [&[u8]; N],
        key: &[u8],
        to_path: Vec<Vec<u8>>,
        element: Element,
        in_tree_type: TreeType,
        estimated_value_size: u32,
        estimating: bool,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        if estimating {
            return self.batch_move(
                from_path.as_slice().into(),
                key,
                to_path,
                BatchMoveApplyType::StatelessBatchMove {
                    in_tree_type,
                    tree_type: None,
                    estimated_key_size: key.len() as u32,
                    estimated_value_size,
                    flags_len: StorageFlags::approximate_size(true, None),
                },
                Some(None),
                transaction,
                batch_operations,
                drive_version,
            );
        }
        self.batch_delete(
            from_path.as_slice().into(),
            key,
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            },
            transaction,
            batch_operations,
            drive_version,
        )?;
        self.batch_insert(
            PathKeyElementInfo::PathKeyElement::<0>((to_path, key.to_vec(), element)),
            batch_operations,
            drive_version,
        )
    }
}
