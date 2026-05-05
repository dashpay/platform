use crate::drive::contract::paths;
use crate::drive::Drive;
use crate::drive::LowLevelDriveOperation;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::common::encode::encode_u64;
use crate::util::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize,
};
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds a contract to storage.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_to_storage_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        insert_operations: &mut Vec<LowLevelDriveOperation>,
        is_first_insert: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let contract_root_path = paths::contract_root_path(contract.id_ref().as_bytes());
        if contract.config().keeps_history() {
            let element_flags = contract_element.get_flags().clone();
            let storage_flags =
                StorageFlags::map_cow_some_element_flags_ref(contract_element.get_flags())?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                    contract,
                    estimated_costs_only_with_layer_info,
                    drive_version,
                )?;
            }

            if is_first_insert {
                self.batch_insert_empty_tree(
                    contract_root_path,
                    KeyRef(&[0]),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    insert_operations,
                    drive_version,
                )?;
            } else {
                let apply_type = if estimated_costs_only_with_layer_info.is_some() {
                    BatchInsertTreeApplyType::StatelessBatchInsertTree {
                        tree_type: TreeType::NormalTree,
                        in_tree_type: TreeType::NormalTree,
                        flags_len: storage_flags
                            .as_ref()
                            .map(|flags| flags.to_element_flags().len())
                            .unwrap_or_default() as u32,
                    }
                } else {
                    BatchInsertTreeApplyType::StatefulBatchInsertTree
                };

                let key_info = PathKeyInfo::PathFixedSizeKeyRef((contract_root_path, &[0]));

                self.batch_insert_empty_tree_if_not_exists(
                    key_info,
                    TreeType::NormalTree,
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    apply_type,
                    transaction,
                    &mut None,
                    insert_operations,
                    drive_version,
                )?;
            }

            let encoded_time = encode_u64(block_info.time_ms);
            let contract_keeping_history_storage_path =
                paths::contract_keeping_history_root_path(contract.id_ref().as_bytes());

            if !is_first_insert {
                // we can use a DirectQueryType::StatefulDirectQuery because if we were stateless we would always think
                // this was the first insert
                let maybe_element = self.grove_get_raw_optional(
                    (&contract_keeping_history_storage_path).into(),
                    encoded_time.as_slice(),
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    insert_operations,
                    drive_version,
                )?;
                if maybe_element.is_some() {
                    return Err(Error::Drive(DriveError::UpdatingContractWithHistoryError(
                        "updating a contract with same time as a previous revision",
                    )));
                }
            };

            self.batch_insert(
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    encoded_time.as_slice(),
                    contract_element,
                )),
                insert_operations,
                drive_version,
            )?;

            let reference_element =
                Element::Reference(SiblingReference(encoded_time), Some(1), element_flags);

            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    &[0],
                    reference_element,
                ))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_keeping_history_storage_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    reference_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations, drive_version)?;
        } else {
            // the contract is just stored at key 0
            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((contract_root_path, &[0], contract_element))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_root_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    contract_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations, drive_version)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;

    /// Exercises the non-history branch of `add_contract_to_storage_v0` with
    /// `estimated_costs_only_with_layer_info` populated (the else path at the
    /// end of the function). This runs a direct stateless insert, which
    /// produces `PathKeyElementSize` ops rather than `PathFixedSizeKeyRefElement`
    /// ops. PR #3516 covers the apply=true (stateful) path extensively; this
    /// targets the estimation branch of the non-history path directly.
    ///
    /// We invoke the public `insert_contract(apply=false)`, which drops into
    /// `insert_contract_element_v0/v1 -> insert_contract_operations_v0` which
    /// in turn invokes `add_contract_to_storage_v0` with
    /// `estimated_costs_only_with_layer_info = Some(..)` — this is the exact
    /// branch that was previously un-exercised for a non-history contract.
    #[test]
    fn test_add_contract_to_storage_v0_non_history_estimate_path() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        // default config has keeps_history=false and readonly=false
        contract.config_mut().set_readonly(false);

        let fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                false, // apply=false drives the PathKeyElementSize estimation branch
                None,
                platform_version,
            )
            .expect("non-history estimate path should succeed");

        assert!(fee.processing_fee > 0 || fee.storage_fee > 0);
    }

    /// Exercises the history-branch estimate path of `add_contract_to_storage_v0`:
    /// - `keeps_history=true`
    /// - `estimated_costs_only_with_layer_info = Some(..)`  (apply=false)
    /// - `is_first_insert=true` (inside `insert_contract_operations_v0`)
    /// This goes through `add_estimation_costs_for_levels_up_to_contract_document_type_excluded`
    /// AND the `PathKeyElementSize` reference branch at the end of the history block.
    #[test]
    fn test_add_contract_to_storage_v0_history_first_insert_estimate_path() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);

        let fee = drive
            .insert_contract(
                &contract,
                BlockInfo {
                    time_ms: 500,
                    height: 1,
                    core_height: 1,
                    epoch: Default::default(),
                },
                false, // estimation
                None,
                platform_version,
            )
            .expect("history+estimate first insert should succeed");

        assert!(fee.processing_fee > 0 || fee.storage_fee > 0);
    }

    /// Also exercises the history branch, but with actual apply (not estimate).
    /// `add_contract_to_storage_v0`'s history path with `is_first_insert=true`
    /// creates the empty history tree and the reference-to-timestamp sibling.
    /// An existing `test_apply_contract_with_history_keeps_history_insert_and_update`
    /// test in mod.rs covers insert+update, but this test specifically targets
    /// the borderline case of a contract with `readonly=true` + `keeps_history=true`,
    /// which drives the `storage_flags = None` path at the top of `insert_contract_v0/v1`
    /// combined with the history branch in `add_contract_to_storage_v0`. This
    /// specific combination (readonly AND history) was not previously covered.
    ///
    /// NOTE: readonly=true requires can_be_deleted=false path; see the storage_flags
    /// computation. This results in `element_flags = None` propagated into
    /// `add_contract_to_storage_v0`, driving the `storage_flags.as_ref().map(..)`
    /// None paths.
    #[test]
    fn test_add_contract_to_storage_v0_history_with_readonly_none_flags() {
        use dpp::data_contract::config::v0::DataContractConfigSettersV0;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(true);
        contract.config_mut().set_can_be_deleted(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 123,
                    height: 1,
                    core_height: 1,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("history+readonly+none-flags insert should succeed");
    }
}
