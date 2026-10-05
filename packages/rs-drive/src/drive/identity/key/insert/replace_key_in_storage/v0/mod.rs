use crate::drive::identity::identity_key_tree_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;
use dpp::serialization::PlatformSerializable;
use grovedb::Element;

impl Drive {
    /// Rewrites a stored key in place. A key is never edited freely on the protocol level: the
    /// only rewrites are disabling a key (`disable_identity_keys`) and raising its limits
    /// (`update_identity_key_limits`), and `change_in_bytes` is the growth of the serialized key
    /// that the rewrite is priced by.
    pub(super) fn replace_key_in_storage_operations_v0(
        &self,
        identity_id: &[u8],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        change_in_bytes: i32,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let serialized_identity_key = identity_key.serialize_to_bytes().map_err(Error::from)?;
        // Now lets insert the public key
        let identity_key_tree = identity_key_tree_path_vec(identity_id);

        drive_operations.push(LowLevelDriveOperation::patch_for_known_path_key_element(
            identity_key_tree,
            key_id_bytes.to_vec(),
            Element::new_item_with_flags(serialized_identity_key, None),
            change_in_bytes,
        ));

        Ok(())
    }
}
