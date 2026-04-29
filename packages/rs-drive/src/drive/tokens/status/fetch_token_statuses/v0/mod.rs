use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_token_statuses_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        self.fetch_token_statuses_operations_v0(
            token_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_token_statuses_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        let path_query = Drive::token_statuses_query(token_ids);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(_, key, element)| {
            let token_id: [u8; 32] = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "token id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(Item(value, ..)) => {
                    Ok((token_id, Some(TokenStatus::deserialize_from_bytes(&value)?)))
                }
                None => Ok((token_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for statuses should contain only items".to_string(),
                ))),
            }
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::tokens::paths::token_statuses_root_path;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::tokens::status::v0::TokenStatusV0;
    use grovedb::Element;

    /// Passing an empty token id list yields an empty BTreeMap without error.
    #[test]
    fn empty_token_id_list_returns_empty_map() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive
            .fetch_token_statuses_v0(&[], None, platform_version)
            .expect("fetch with no ids");
        assert!(result.is_empty());
    }

    /// Mixed existing + non-existing tokens populate the map with Some / None
    /// entries in a single call.
    #[test]
    fn fetch_mixed_existing_and_missing_tokens() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_1 = [0xA1u8; 32];
        let missing = [0xA2u8; 32];

        let paused = TokenStatus::V0(TokenStatusV0 { paused: true });
        drive
            .token_apply_status(
                token_1,
                paused.clone(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("apply status 1");

        let result = drive
            .fetch_token_statuses_v0(&[token_1, missing], None, platform_version)
            .expect("fetch both");
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&token_1).cloned(), Some(Some(paused)));
        assert_eq!(result.get(&missing).cloned(), Some(None));
    }

    /// Storing a non-Item element under a token_id key triggers the
    /// "token tree for statuses should contain only items" branch.
    #[test]
    fn fetch_statuses_rejects_non_item_element() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0xBBu8; 32];
        let path = token_statuses_root_path();

        drive
            .grove
            .insert(
                path.as_ref(),
                &token_id,
                Element::empty_tree(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("insert wrong element");

        let err = drive
            .fetch_token_statuses_v0(&[token_id], None, platform_version)
            .expect_err("fetch should reject non-item in the statuses tree");
        match err {
            Error::Drive(DriveError::CorruptedDriveState(msg)) => {
                assert!(msg.contains("items"));
            }
            other => panic!("expected CorruptedDriveState, got {:?}", other),
        }
    }
}
