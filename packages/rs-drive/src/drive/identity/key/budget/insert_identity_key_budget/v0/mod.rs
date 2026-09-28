use crate::drive::identity::identity_path_vec;
use crate::drive::identity::key::budget::identity_key_budgets_path_vec;
use crate::drive::identity::IdentityRootStructure::IdentityTreeKeyBudgets;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::PathKeyInfo;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::IdentityPublicKey;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use integer_encoding::VarInt;
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn insert_identity_key_budget_operations_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let Some(total_budget) = identity_key.total_budget() else {
            return Ok(());
        };

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_key_budgets(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_type: TreeType::NormalTree,
                tree_type: TreeType::NormalTree,
                flags_len: 0,
            }
        };

        // The subtree is not part of a new identity, and identities created before budgets
        // existed do not have it, so it is created with the first budgeted key. Several budgeted
        // keys may arrive in one state transition, hence the check against the pending
        // operations as well as the state.
        self.batch_insert_empty_tree_if_not_exists_check_existing_operations(
            PathKeyInfo::<0>::PathKey((
                identity_path_vec(identity_id.as_slice()),
                vec![IdentityTreeKeyBudgets as u8],
            )),
            false,
            None,
            apply_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        // Fixed width, so that spending from the budget replaces the value without changing
        // what is stored. Key ids are never reused, so the slot is always new.
        self.batch_insert(
            PathKeyElement::<0>((
                identity_key_budgets_path_vec(identity_id.as_slice()),
                identity_key.id().encode_var_vec(),
                Element::new_item(total_budget.to_be_bytes().to_vec()),
            )),
            drive_operations,
            &platform_version.drive,
        )
    }
}
