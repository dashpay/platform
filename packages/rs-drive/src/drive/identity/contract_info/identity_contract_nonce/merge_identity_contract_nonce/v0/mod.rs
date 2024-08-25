use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{
    identity_contract_info_group_path, identity_contract_info_root_path_vec, identity_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::{PathKeyElementInfo, PathKeyInfo};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::identity_nonce::{IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES, MAX_MISSING_IDENTITY_REVISIONS, MISSING_IDENTITY_REVISIONS_FILTER, MISSING_IDENTITY_REVISIONS_MAX_BYTES};
use dpp::prelude::IdentityNonce;
use crate::drive::identity::contract_info::identity_contract_nonce::merge_identity_contract_nonce::MergeIdentityNonceResult;
use crate::drive::identity::contract_info::identity_contract_nonce::merge_identity_contract_nonce::MergeIdentityNonceResult::{MergeIdentityNonceSuccess, NonceAlreadyPresentAtTip, NonceAlreadyPresentInPast, NonceTooFarInFuture, NonceTooFarInPast};

impl Drive {
    pub(in crate::drive::identity::contract_info) fn merge_identity_contract_nonce_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        identity_contract_nonce: IdentityNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<MergeIdentityNonceResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let (result, batch_operations) = self.merge_identity_contract_nonce_operations_v0(
            identity_id,
            contract_id,
            identity_contract_nonce,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        if !result.is_error() {
            self.apply_batch_low_level_drive_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                drive_operations,
                &platform_version.drive,
            )?;
        }

        Ok(result)
    }

    /// Sets the revision nonce for the identity contract pair
    pub(super) fn merge_identity_contract_nonce_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityNonce,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(MergeIdentityNonceResult, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];

        if revision_nonce & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
            return Ok((MergeIdentityNonceResult::InvalidNonce, drive_operations));
        }

        if revision_nonce == 0 {
            return Ok((MergeIdentityNonceResult::InvalidNonce, drive_operations));
        }

        let identity_path = identity_path_vec(identity_id.as_slice());

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info(
                &identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: 0,
            }
        };

        let previous_nonce_is_sure_to_not_exist = if revision_nonce
            <= MAX_MISSING_IDENTITY_REVISIONS
        {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_contract_info(
                    &identity_id,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            // we insert the contract root tree if it doesn't exist already
            self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
                false,
                None,
                apply_type,
                transaction,
                &mut None,
                &mut drive_operations,
                &platform_version.drive,
            )?;

            // we insert the contract root tree if it doesn't exist already
            let inserted = self.batch_insert_empty_tree_if_not_exists(
                PathKeyInfo::<0>::PathKey((
                    identity_contract_info_root_path_vec(&identity_id),
                    contract_id.to_vec(),
                )),
                false,
                None,
                apply_type,
                transaction,
                &mut None,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            if estimated_costs_only_with_layer_info.is_none() {
                inserted
            } else {
                false //in the case of fee estimation it might exist
            }
        } else {
            false
        };

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info_group(
                &identity_id,
                &contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let (existing_nonce, _fees) = if previous_nonce_is_sure_to_not_exist {
            (None, FeeResult::default())
        } else {
            self.fetch_identity_contract_nonce_with_fees(
                identity_id,
                contract_id,
                block_info,
                estimated_costs_only_with_layer_info.is_none(),
                transaction,
                platform_version,
            )?
        };

        let (nonce_to_set, is_new) = if estimated_costs_only_with_layer_info.is_some() {
            // we are just getting estimated costs
            (revision_nonce, true)
        } else if let Some(existing_nonce) = existing_nonce {
            let actual_existing_revision = existing_nonce & IDENTITY_NONCE_VALUE_FILTER;
            let nonce_to_set = match actual_existing_revision.cmp(&revision_nonce) {
                std::cmp::Ordering::Equal => {
                    // we were not able to update the revision as it is the same as we already had
                    return Ok((NonceAlreadyPresentAtTip, drive_operations));
                }
                std::cmp::Ordering::Less => {
                    if revision_nonce - actual_existing_revision
                        > MISSING_IDENTITY_REVISIONS_MAX_BYTES
                    {
                        // we are too far away from the actual revision
                        return Ok((NonceTooFarInFuture, drive_operations));
                    } else {
                        let missing_amount_of_revisions =
                            revision_nonce - actual_existing_revision - 1;
                        let new_previous_missing_revisions = (existing_nonce
                            & MISSING_IDENTITY_REVISIONS_FILTER)
                            << (missing_amount_of_revisions + 1);
                        // the missing_revisions_bytes are the amount of bytes to put in the missing area
                        let missing_revisions_bytes = if missing_amount_of_revisions > 0 {
                            ((1 << missing_amount_of_revisions) - 1)
                                << IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES
                        } else {
                            0
                        };
                        new_previous_missing_revisions | revision_nonce | missing_revisions_bytes
                    }
                }
                std::cmp::Ordering::Greater => {
                    let previous_revision_position_from_top =
                        actual_existing_revision - revision_nonce;
                    if previous_revision_position_from_top >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
                        // we are too far away from the actual revision
                        return Ok((NonceTooFarInPast, drive_operations));
                    } else {
                        let old_missing_revisions =
                            existing_nonce & MISSING_IDENTITY_REVISIONS_FILTER;
                        if old_missing_revisions == 0 {
                            return Ok((
                                NonceAlreadyPresentInPast(previous_revision_position_from_top),
                                drive_operations,
                            ));
                        } else {
                            let byte_to_unset = 1
                                << (previous_revision_position_from_top - 1
                                    + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES);
                            let old_revision_already_set =
                                old_missing_revisions | byte_to_unset != old_missing_revisions;
                            if old_revision_already_set {
                                return Ok((
                                    NonceAlreadyPresentInPast(previous_revision_position_from_top),
                                    drive_operations,
                                ));
                            } else {
                                existing_nonce & !byte_to_unset
                            }
                        }
                    }
                }
            };
            (nonce_to_set, false)
        } else if revision_nonce >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
            // we are too far away from the actual revision
            return Ok((NonceTooFarInFuture, drive_operations));
        } else {
            // there was no previous revision nonce, just set
            // todo: this will only work if we have at most one document per state transition
            //  when we change batch state transitions back to multiple we need to check existing
            //  operations.

            let missing_amount_of_revisions = revision_nonce - 1;
            // the missing_revisions_bytes are the amount of bytes to put in the missing area
            let missing_revisions_bytes = if missing_amount_of_revisions > 0 {
                ((1 << missing_amount_of_revisions) - 1) << IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES
            } else {
                0
            };

            (missing_revisions_bytes | revision_nonce, true)
        };

        let identity_contract_nonce_bytes = nonce_to_set.to_be_bytes().to_vec();
        let identity_contract_nonce_element = Element::new_item(identity_contract_nonce_bytes);

        //println!("{} is {:b}, existing was {:?}", nonce_to_set,  nonce_to_set, existing_nonce);

        if is_new {
            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    identity_contract_info_group_path(&identity_id, &contract_id),
                    &[IdentityContractNonceKey as u8],
                    identity_contract_nonce_element,
                )),
                &mut drive_operations,
                &platform_version.drive,
            )?;
        } else {
            // We are replacing the nonce, matters for fees
            self.batch_replace(
                PathKeyElementInfo::PathFixedSizeKeyRefElement((
                    identity_contract_info_group_path(&identity_id, &contract_id),
                    &[IdentityContractNonceKey as u8],
                    identity_contract_nonce_element,
                )),
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }

        Ok((MergeIdentityNonceSuccess(nonce_to_set), drive_operations))
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::IdentityNonce;
    use platform_version::version::PlatformVersion;

    fn setup_base_test(contract_id: [u8; 32]) -> (Drive, Identity) {
        let drive = setup_drive(None);
        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                1,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        drive
            .commit_transaction(transaction, &platform_version.drive)
            .expect("expected to commit transaction");

        (drive, identity)
    }

    #[test]
    fn merge_identity_contract_nonce_with_bump() {
        let contract_id = [0; 32];
        let (drive, identity) = setup_base_test(contract_id);

        let platform_version = PlatformVersion::first();

        let transaction = drive.grove.start_transaction();

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                2,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());
    }

    #[test]
    fn merge_identity_contract_nonce_0_is_invalid() {
        let contract_id = [0; 32];
        let (drive, identity) = setup_base_test(contract_id);

        let platform_version = PlatformVersion::first();

        let transaction = drive.grove.start_transaction();

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                0,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce is an invalid value"));
    }

    #[test]
    fn merge_identity_contract_nonce_many_updates() {
        let contract_id = [0; 32];
        let (drive, identity) = setup_base_test(contract_id);

        let platform_version = PlatformVersion::first();

        let transaction = drive.grove.start_transaction();

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                10,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                9,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                8,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                3,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                12,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                11,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                11,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(
            result.error_message(),
            Some("nonce already present in past")
        );

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                12,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce already present at tip"));

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                0,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce is an invalid value"));

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                12 + 25,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce too far in future"));

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                12 + 24, // 36
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                13,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert!(result.error_message().is_none());

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                12,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce too far in past"));

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                8,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce too far in past"));

        let result = drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                IdentityNonce::MAX,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut vec![],
                platform_version,
            )
            .expect("expected to merge identity contract nonce");

        assert_eq!(result.error_message(), Some("nonce is an invalid value"));
    }
}
