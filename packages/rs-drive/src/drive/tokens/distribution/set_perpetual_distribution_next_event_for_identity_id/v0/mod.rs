use crate::drive::tokens::paths::token_perpetual_distributions_next_not_done_event_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::Element;
use crate::util::object_size_info::PathKeyElementInfo;

impl Drive {
    /// Sets the next scheduled event time for a perpetual distribution for a given identity.
    ///
    /// This method updates the tree at `token_perpetual_distributions_path_vec(token_id)`
    /// by storing an 8-byte big-endian encoded timestamp representing the next scheduled distribution event.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose next event timestamp is being set.
    /// - `next_event_time`: The `TimestampMillis` indicating the next scheduled distribution.
    /// - `block_info`: Block metadata for setting storage flags.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result<(), Error>` indicating success or failure.
    pub(super) fn set_perpetual_distribution_next_event_for_identity_id_v0(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        next_event_time: TimestampMillis,
        block_info: &BlockInfo,
        known_to_be_replace: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let perpetual_distributions_path = token_perpetual_distributions_next_not_done_event_path_vec(token_id);

        // Convert the next event time to a big-endian 8-byte array
        let next_event_bytes = next_event_time.to_be_bytes().to_vec();

        // Generate storage flags for tracking historical cleanup
        let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(identity_id.to_buffer()));
        
        if known_to_be_replace {
            // This is slightly more performant
            self.batch_replace(
                PathKeyElementInfo::<0>::PathKeyRefElement((perpetual_distributions_path, identity_id.as_slice(), Element::new_item_with_flags(next_event_bytes, storage_flags.to_some_element_flags()))),
                drive_operations,
                &platform_version.drive,
            )?;
        } else {
            // Insert the timestamp into the tree
            self.batch_insert(
                PathKeyElementInfo::<0>::PathKeyRefElement((perpetual_distributions_path, identity_id.as_slice(), Element::new_item_with_flags(next_event_bytes, storage_flags.to_some_element_flags()))),
                drive_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}