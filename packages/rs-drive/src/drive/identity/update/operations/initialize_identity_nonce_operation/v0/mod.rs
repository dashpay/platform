use crate::drive::identity::{identity_path_vec, IdentityRootStructure};
use crate::drive::Drive;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::IdentityNonce;
use grovedb::Element;

impl Drive {
    /// Initialize the nonce of the identity, should only be called on create identity
    /// Nonces get bumped on all identity state transitions except those that use an asset lock
    pub(in crate::drive::identity) fn initialize_identity_nonce_operation_v0(
        &self,
        identity_id: [u8; 32],
        nonce: IdentityNonce,
    ) -> LowLevelDriveOperation {
        let identity_path = identity_path_vec(identity_id.as_slice());
        let revision_bytes = nonce.to_be_bytes().to_vec();
        LowLevelDriveOperation::insert_for_known_path_key_element(
            identity_path,
            Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeNonce).to_vec(),
            Element::new_item(revision_bytes),
        )
    }
}
