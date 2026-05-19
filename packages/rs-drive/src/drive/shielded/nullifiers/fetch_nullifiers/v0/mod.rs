use crate::drive::shielded::nullifiers::queries::shielded_recent_nullifiers_path_vec;
use crate::drive::shielded::nullifiers::types::{CompactedNullifiers, NullifierChangePerBlock};
use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of fetching nullifier changes from a start height.
    ///
    /// Retrieves all nullifier change records from `start_height` onwards.
    /// Returns a vector of (block_height, nullifiers) tuples.
    pub(in crate::drive) fn fetch_recent_nullifier_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<NullifierChangePerBlock>, Error> {
        let path = shielded_recent_nullifiers_path_vec();

        // Create a range query starting from the specified height
        let mut query = Query::new();
        query.insert_range_from(start_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let mut nullifier_changes = Vec::new();

        for (key, element) in results.to_key_elements() {
            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                    "invalid block height key length".to_string(),
                )))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item with sum item element for nullifiers".to_string(),
                    ),
                )));
            };

            // Deserialize the nullifier list
            let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            nullifier_changes.push(NullifierChangePerBlock {
                block_height,
                nullifiers,
            });
        }

        Ok(nullifier_changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::shielded::nullifiers::queries::shielded_recent_nullifiers_path_vec;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::ProtocolError;

    /// Fetching from an empty nullifier pool returns an empty vec (not an error).
    #[test]
    fn fetch_empty_pool_returns_empty_vec() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_recent_nullifier_changes_v0(0, None, None, platform_version)
            .expect("fetch should succeed");
        assert!(result.is_empty());
    }

    /// Fetch honors a non-zero start_height: blocks before start_height are skipped.
    #[test]
    fn fetch_skips_blocks_before_start_height() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Store 3 blocks.
        for height in [10u64, 20, 30] {
            drive
                .store_nullifiers_for_block_v0(
                    &[[height as u8; 32]],
                    height,
                    1_000 * height,
                    None,
                    platform_version,
                )
                .expect("store");
        }

        let from_25 = drive
            .fetch_recent_nullifier_changes_v0(25, None, None, platform_version)
            .expect("fetch from 25");
        assert_eq!(from_25.len(), 1, "only block 30 should be returned");
        assert_eq!(from_25[0].block_height, 30);
    }

    /// Fetch honors the limit parameter. With limit=2 and 3 stored blocks, we get 2.
    #[test]
    fn fetch_honors_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for height in [1u64, 2, 3] {
            drive
                .store_nullifiers_for_block_v0(
                    &[[height as u8; 32]],
                    height,
                    1_000,
                    None,
                    platform_version,
                )
                .expect("store");
        }

        let limited = drive
            .fetch_recent_nullifier_changes_v0(0, Some(2), None, platform_version)
            .expect("fetch with limit");
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].block_height, 1);
        assert_eq!(limited[1].block_height, 2);
    }

    /// Directly inserting a malformed (wrong-type) element into the nullifiers tree
    /// triggers the CorruptedSerialization branch in the fetch loop.
    #[test]
    fn fetch_rejects_non_item_with_sum_item_element() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert a plain Item element directly (bypassing the store API). fetch expects
        // ItemWithSumItem and must error on plain Item.
        let path = shielded_recent_nullifiers_path_vec();
        let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
        let key = 1u64.to_be_bytes();
        drive
            .grove
            .insert(
                path_refs.as_slice(),
                &key,
                grovedb::Element::new_item(vec![0u8; 4]),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("force-insert plain item");

        match drive.fetch_recent_nullifier_changes_v0(0, None, None, platform_version) {
            Err(Error::Protocol(b)) => match b.as_ref() {
                ProtocolError::CorruptedSerialization(_) => {}
                other => panic!("expected CorruptedSerialization, got: {:?}", other),
            },
            Err(other) => panic!("expected Error::Protocol, got: {:?}", other),
            Ok(_) => panic!("fetch should detect the wrong element kind"),
        }
    }
}
