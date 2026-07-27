use crate::drive::tokens::paths::token_statuses_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_token_status_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        self.fetch_token_status_operations_v0(
            token_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_token_status_operations_v0(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let token_statuses_root_path = token_statuses_root_path();

        match self.grove_get_raw_optional(
            (&token_statuses_root_path).into(),
            &token_id,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(info, _))) => {
                Ok(Some(TokenStatus::deserialize_from_bytes(info.as_slice())?))
            }

            Ok(None) => Ok(None),
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                Ok(None)
            }

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "token status was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
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

    /// Fetching a token status for a token that was never inserted returns None
    /// (exercises the PathKeyNotFound-swallowed branch).
    #[test]
    fn fetch_nonexistent_token_status_returns_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let missing_id = [0xCCu8; 32];
        let result = drive
            .fetch_token_status_v0(missing_id, None, platform_version)
            .expect("fetch missing token should return Ok(None)");
        assert_eq!(result, None);
    }

    /// Round-trip: apply a paused status, fetch it back, then apply an unpaused
    /// status and verify the state transition replaces the previous value.
    #[test]
    fn paused_to_active_transition_round_trips() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0x11u8; 32];

        // Step 1: set paused = true.
        let paused = TokenStatus::V0(TokenStatusV0 { paused: true });
        drive
            .token_apply_status(
                token_id,
                paused.clone(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("apply paused");

        let fetched = drive
            .fetch_token_status_v0(token_id, None, platform_version)
            .expect("fetch after paused");
        assert_eq!(fetched, Some(paused));

        // Step 2: flip back to paused = false (active).
        let active = TokenStatus::V0(TokenStatusV0 { paused: false });
        drive
            .token_apply_status(
                token_id,
                active.clone(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("apply active");

        let fetched = drive
            .fetch_token_status_v0(token_id, None, platform_version)
            .expect("fetch after active");
        assert_eq!(fetched, Some(active));
    }

    /// Stateless (apply=false) fetch must not require the element to exist — it
    /// just reports estimated costs and returns None (exercising the stateless
    /// DirectQuery branch in fetch_token_status_operations_v0).
    #[test]
    fn stateless_fetch_returns_none_for_missing_token_without_error() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let missing_id = [0xCDu8; 32];
        let mut drive_operations = vec![];
        let result = drive
            .fetch_token_status_operations_v0(
                missing_id,
                false, // stateless
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("stateless fetch should succeed");
        // Stateless mode returns None; the DriveOperations vec captures cost info.
        assert_eq!(result, None);
    }

    /// Storing a non-Item element under a token_id key triggers the
    /// "CorruptedElementType" branch when fetch expects Item.
    #[test]
    fn fetch_rejects_non_item_element() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0x77u8; 32];
        let path = token_statuses_root_path();

        // Force-insert an empty subtree instead of an Item.
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
            .expect("insert wrong element type");

        let err = drive
            .fetch_token_status_v0(token_id, None, platform_version)
            .expect_err("fetch should reject non-Item element");
        match err {
            Error::Drive(DriveError::CorruptedElementType(_)) => {}
            other => panic!("expected CorruptedElementType, got {:?}", other),
        }
    }

    /// Storing a garbage item payload triggers TokenStatus::deserialize_from_bytes
    /// to error out. This exercises the deserialization-error propagation path.
    #[test]
    fn fetch_rejects_undecodable_token_status_item() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0x88u8; 32];
        let path = token_statuses_root_path();

        // Insert a junk item — not a valid serialized TokenStatus.
        drive
            .grove
            .insert(
                path.as_ref(),
                &token_id,
                Element::new_item(vec![0xFFu8; 2]),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("insert garbage item");

        let err = drive
            .fetch_token_status_v0(token_id, None, platform_version)
            .expect_err("fetch should fail on garbage item");
        // Allow any deserialization error variant; the important behavior is
        // that fetch does not silently succeed.
        let _ = err;
    }
}
