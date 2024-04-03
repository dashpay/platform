use crate::drive::identity::identity_key_tree_path;
use crate::drive::object_size_info::PathKeyElementInfo::PathFixedSizeKeyRefElement;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;
use dpp::serialization::PlatformSerializable;
use grovedb::Element;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Generates a vector of operations for inserting key to storage.
    #[inline(always)]
    pub(super) fn insert_key_to_storage_operations_v0(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        key_id_bytes: &[u8],
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let serialized_identity_key = identity_key.serialize_to_bytes().map_err(Error::Protocol)?;
        // Now lets insert the public key
        let identity_key_tree = identity_key_tree_path(identity_id.as_slice());

        self.batch_insert(
            PathFixedSizeKeyRefElement((
                identity_key_tree,
                key_id_bytes,
                Element::new_item_with_flags(serialized_identity_key, None),
            )),
            drive_operations,
            &platform_version.drive,
        )
    }
}
