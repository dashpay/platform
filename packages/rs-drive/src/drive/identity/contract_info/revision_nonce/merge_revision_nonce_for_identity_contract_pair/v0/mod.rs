use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use crate::drive::identity::IdentityRootStructure::IdentityContractInfo;
use crate::drive::identity::{identity_contract_info_group_path, identity_path_vec};
use crate::drive::object_size_info::{PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::IdentityContractNonce;
use crate::drive::identity::contract_info::revision_nonce::merge_revision_nonce_for_identity_contract_pair::MergeIdentityContractNonceResult;
use crate::drive::identity::contract_info::revision_nonce::merge_revision_nonce_for_identity_contract_pair::MergeIdentityContractNonceResult::{MergeIdentityContractNonceSuccess, NonceAlreadyPresentAtTip, NonceAlreadyPresentInPast, NonceTooFarInFuture, NonceTooFarInPast};
use crate::error::identity::IdentityError;

const VALUE_FILTER: u64 = 0xFFFFF;
const MISSING_REVISIONS_FILTER: u64 = 0xFFF00000;
const MISSING_REVISIONS_MAX_BYTES: u64 = 20;
const VALUE_FILTER_MAX_BYTES: u64 = 40;

impl Drive {
    pub(in crate::drive::identity::contract_info) fn merge_revision_nonce_for_identity_contract_pair_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityContractNonce,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<MergeIdentityContractNonceResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let (result, batch_operations) = self
            .merge_revision_nonce_for_identity_contract_pair_operations_v0(
                identity_id,
                contract_id,
                revision_nonce,
                block_info,
                &mut estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )?;
        Ok(result)
    }

    /// Sets the revision nonce for the identity contract pair
    pub(super) fn merge_revision_nonce_for_identity_contract_pair_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        revision_nonce: IdentityContractNonce,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            MergeIdentityContractNonceResult,
            Vec<LowLevelDriveOperation>,
        ),
        Error,
    > {
        if revision_nonce & MISSING_REVISIONS_FILTER > 0 {
            return Err(Error::Identity(
                IdentityError::IdentityContractRevisionNonceError(
                    "revision nonce was set too high or with missing revision bytes",
                ),
            ));
        }

        let mut drive_operations = vec![];

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

        // we insert the contract root tree if it doesn't exist already
        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::<0>::PathKey((identity_path, vec![IdentityContractInfo as u8])),
            None,
            apply_type,
            transaction,
            &mut None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_info_group(
                &identity_id,
                &contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let (existing_nonce, fees) = self.fetch_identity_contract_nonce_with_fees(
            identity_id,
            contract_id,
            block_info,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            platform_version,
        )?;

        let nonce_to_set = if let Some(existing_nonce) = existing_nonce {
            let actual_existing_revision = existing_nonce & VALUE_FILTER;
            if actual_existing_revision == revision_nonce {
                // we were not able to update the revision as it is the same as we already had
                return Ok((NonceAlreadyPresentAtTip, drive_operations));
            } else if actual_existing_revision < revision_nonce {
                if revision_nonce - actual_existing_revision >= MISSING_REVISIONS_MAX_BYTES {
                    // we are too far away from the actual revision
                    return Ok((NonceTooFarInFuture, drive_operations));
                } else {
                    let missing_amount_of_revisions = revision_nonce - actual_existing_revision;
                    let new_previous_missing_revisions =
                        (existing_nonce & MISSING_REVISIONS_FILTER) << missing_amount_of_revisions;
                    new_previous_missing_revisions | revision_nonce
                }
            } else {
                let previous_revision_position_from_top = actual_existing_revision - revision_nonce;
                if previous_revision_position_from_top >= MISSING_REVISIONS_MAX_BYTES {
                    // we are too far away from the actual revision
                    return Ok((NonceTooFarInPast, drive_operations));
                } else {
                    let old_missing_revisions = (existing_nonce & MISSING_REVISIONS_FILTER);
                    let byte_to_set =
                        1 << (previous_revision_position_from_top + VALUE_FILTER_MAX_BYTES);
                    let old_revision_already_set = (old_missing_revisions & byte_to_set) > 0;
                    if old_revision_already_set {
                        return Ok((
                            NonceAlreadyPresentInPast(previous_revision_position_from_top),
                            drive_operations,
                        ));
                    } else {
                        existing_nonce & byte_to_set
                    }
                }
            }
        } else if revision_nonce >= MISSING_REVISIONS_MAX_BYTES {
            // we are too far away from the actual revision
            return Ok((NonceTooFarInFuture, drive_operations));
        } else {
            // there was no previous revision nonce, just set
            // todo: this will only work if we have at most one document per state transition
            //  when we change batch state transitions back to multiple we need to check existing
            //  operations.
            revision_nonce
        };

        let identity_contract_nonce_bytes = nonce_to_set.to_be_bytes().to_vec();
        let identity_contract_nonce_element = Element::new_item(identity_contract_nonce_bytes);

        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                identity_contract_info_group_path(&identity_id, &contract_id),
                &[IdentityContractNonceKey as u8],
                identity_contract_nonce_element,
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok((
            MergeIdentityContractNonceSuccess(nonce_to_set),
            drive_operations,
        ))
    }
}
