use dpp::block::extended_block_info::BlockInfo;

use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::batch::KeyInfoPath;

use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairVec, KeyRequestType,
};
use crate::fee::result::FeeResult;

use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Revision, TimestampMillis};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use integer_encoding::VarInt;
use std::collections::HashMap;
use dpp::state_transition::fee::calculate_fee;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Update the revision of the identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(super) fn update_identity_revision_operation_v0(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> LowLevelDriveOperation {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_update_revision(
                identity_id,
                estimated_costs_only_with_layer_info,
            );
        }
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = revision.to_be_bytes().to_vec();
        LowLevelDriveOperation::replace_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeRevision).to_vec(),
            Element::new_item(revision_bytes),
        )
    }
}