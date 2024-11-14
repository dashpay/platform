use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::Revision;
use grovedb::Element;

impl Drive {
    /// Initialize the revision of the identity, should only be called on create identity
    /// Revisions get bumped on all changes except for the balance and negative credit fields
    pub(in crate::drive::identity) fn initialize_identity_revision_operation_v0(
        &self,
        identity_id: [u8; 32],
        revision: Revision,
    ) -> LowLevelDriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = revision.to_be_bytes().to_vec();
        LowLevelDriveOperation::insert_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeRevision).to_vec(),
            Element::new_item(revision_bytes),
        )
    }
}
