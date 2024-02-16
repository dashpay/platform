use crate::drive::identity::{identity_path, identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;

use crate::error::Error;

use crate::fee::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use dpp::prelude::IdentityNonce;

use dpp::version::PlatformVersion;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use crate::drive::identity::IdentityRootStructure::IdentityTreeNonce;
use crate::drive::object_size_info::PathKeyElementInfo;
use crate::error::identity::IdentityError;
use dpp::block::block_info::BlockInfo;
use dpp::identity::identity_nonce::MergeIdentityNonceResult::{
    MergeIdentityNonceSuccess, NonceAlreadyPresentAtTip, NonceAlreadyPresentInPast,
    NonceTooFarInFuture, NonceTooFarInPast,
};
use dpp::identity::identity_nonce::{
    MergeIdentityNonceResult, IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
    MAX_MISSING_IDENTITY_REVISIONS, MISSING_IDENTITY_REVISIONS_FILTER,
    MISSING_IDENTITY_REVISIONS_MAX_BYTES,
};
use std::collections::HashMap;

impl Drive {
    /// Update the nonce of the identity
    /// Nonces get bumped on all identity state transitions except those that use an asset lock
    pub(in crate::drive::identity::update) fn merge_identity_nonce_operations_v0(
        &self,
        identity_id: [u8; 32],
        revision_nonce: IdentityNonce,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(MergeIdentityNonceResult, Vec<LowLevelDriveOperation>), Error> {
        if revision_nonce & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
            return Err(Error::Identity(IdentityError::IdentityNonceError(
                "revision nonce was set too high or with missing revision bytes",
            )));
        }

        if revision_nonce == 0 {
            return Err(Error::Identity(IdentityError::IdentityNonceError(
                "revision nonce must not be 0",
            )));
        }

        let mut drive_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_update_nonce(
                identity_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let (existing_nonce, fees) = self.fetch_identity_nonce_with_fees(
            identity_id,
            block_info,
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            platform_version,
        )?;

        let nonce_to_set = if estimated_costs_only_with_layer_info.is_some() {
            // we are just getting estimated costs
            revision_nonce
        } else if let Some(existing_nonce) = existing_nonce {
            let actual_existing_revision = existing_nonce & IDENTITY_NONCE_VALUE_FILTER;
            if actual_existing_revision == revision_nonce {
                // we were not able to update the revision as it is the same as we already had
                return Ok((NonceAlreadyPresentAtTip, drive_operations));
            } else if actual_existing_revision < revision_nonce {
                if revision_nonce - actual_existing_revision >= MISSING_IDENTITY_REVISIONS_MAX_BYTES
                {
                    // we are too far away from the actual revision
                    return Ok((NonceTooFarInFuture, drive_operations));
                } else {
                    let missing_amount_of_revisions = revision_nonce - actual_existing_revision - 1;
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
            } else {
                let previous_revision_position_from_top = actual_existing_revision - revision_nonce;
                if previous_revision_position_from_top >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
                    // we are too far away from the actual revision
                    return Ok((NonceTooFarInPast, drive_operations));
                } else {
                    let old_missing_revisions = existing_nonce & MISSING_IDENTITY_REVISIONS_FILTER;
                    let byte_to_unset = 1
                        << (previous_revision_position_from_top - 1
                            + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES);
                    let old_revision_already_existing =
                        (old_missing_revisions & !byte_to_unset) > 0;
                    if old_revision_already_existing {
                        return Ok((
                            NonceAlreadyPresentInPast(previous_revision_position_from_top),
                            drive_operations,
                        ));
                    } else {
                        existing_nonce & !byte_to_unset
                    }
                }
            }
        } else if revision_nonce >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
            // we are too far away from the actual revision
            return Ok((NonceTooFarInFuture, drive_operations));
        } else {
            // there was no previous revision nonce, just set it

            let missing_amount_of_revisions = revision_nonce - 1;
            // the missing_revisions_bytes are the amount of bytes to put in the missing area
            let missing_revisions_bytes = if missing_amount_of_revisions > 0 {
                ((1 << missing_amount_of_revisions) - 1) << IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES
            } else {
                0
            };

            missing_revisions_bytes | revision_nonce
        };

        let identity_contract_nonce_bytes = nonce_to_set.to_be_bytes().to_vec();
        let identity_contract_nonce_element = Element::new_item(identity_contract_nonce_bytes);

        //println!("{} is {:b}, existing was {:?}", nonce_to_set,  nonce_to_set, existing_nonce);

        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                identity_path(&identity_id),
                &[IdentityTreeNonce as u8],
                identity_contract_nonce_element,
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok((MergeIdentityNonceSuccess(nonce_to_set), drive_operations))
    }
}
