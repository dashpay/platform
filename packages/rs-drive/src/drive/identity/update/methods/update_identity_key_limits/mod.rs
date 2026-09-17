mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::identity::{KeyID, TimestampMillis};
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Raises the limits of one of an identity's keys and applies the change: the key is
    /// rewritten with the new total budget and expiry, and its remaining budget grows by the
    /// amount the total budget grew. Consensus decides beforehand that the key exists, has the
    /// limits being raised, and that each new value is greater than the current one.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity the key belongs to.
    /// * `key_id` - The key whose limits are raised.
    /// * `total_budget` - The new total budget, `None` to leave it as it is.
    /// * `expires_at` - The new expiry, `None` to leave it as it is.
    /// * `block_info` - The current block information.
    /// * `apply` - Whether to apply the change or only estimate its fee.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The resulting fee if successful, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn update_identity_key_limits(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .update_identity_key_limits
        {
            Some(0) => self.update_identity_key_limits_v0(
                identity_id,
                key_id,
                total_budget,
                expires_at,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_identity_key_limits".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "update_identity_key_limits".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations that raise the limits of one of an identity's keys. This function is
    /// version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity the key belongs to.
    /// * `key_id` - The key whose limits are raised.
    /// * `total_budget` - The new total budget, `None` to leave it as it is.
    /// * `expires_at` - The new expiry, `None` to leave it as it is.
    /// * `epoch` - The current epoch.
    /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn update_identity_key_limits_operations(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .update_identity_key_limits
        {
            Some(0) => self.update_identity_key_limits_operations_v0(
                identity_id,
                key_id,
                total_budget,
                expires_at,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_identity_key_limits_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "update_identity_key_limits_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::key::fetch::{
        IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap,
    };
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
    use dpp::version::PlatformVersion;

    const KEY_ID: KeyID = 5;
    /// Five bytes as a bincode varint
    const BUDGET: Credits = 1_000_000;
    /// Nine bytes as a bincode varint: the stored key grows when the budget is raised to it
    const RAISED_BUDGET: Credits = 5_000_000_000;
    const EXPIRES_AT: TimestampMillis = 2_000_000;

    fn block() -> BlockInfo {
        BlockInfo::default_with_epoch(Epoch::new(0).expect("expected epoch 0"))
    }

    /// A drive holding an identity with ordinary keys 0 to 4 and key 5 with the given limits.
    fn setup(
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> (Drive, [u8; 32]) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");
        let identity_id = identity.id().to_buffer();
        let key =
            IdentityPublicKey::random_authentication_keys(KEY_ID, 1, Some(15), platform_version)
                .remove(0)
                .with_limits(total_budget, expires_at);
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![key],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the limited key");
        (drive, identity_id)
    }

    fn stored_key(drive: &Drive, identity_id: [u8; 32]) -> IdentityPublicKey {
        drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                IdentityKeysRequest::new_specific_key_query(&identity_id, KEY_ID),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the key")
            .remove(&KEY_ID)
            .expect("expected the key to be stored")
    }

    fn remaining_budget(drive: &Drive, identity_id: [u8; 32]) -> Option<Credits> {
        drive
            .fetch_identity_key_remaining_budget(
                identity_id,
                KEY_ID,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the remaining budget")
    }

    #[test]
    fn should_raise_the_total_and_the_remaining_budget_by_the_same_amount() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(Some(BUDGET), Some(EXPIRES_AT));
        drive
            .deduct_from_identity_key_budget(identity_id, KEY_ID, 300_000, None, platform_version)
            .expect("expected to spend from the budget");
        assert_eq!(remaining_budget(&drive, identity_id), Some(700_000));

        let fee = drive
            .update_identity_key_limits(
                identity_id,
                KEY_ID,
                Some(RAISED_BUDGET),
                None,
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to raise the budget");
        assert!(fee.storage_fee > 0, "the stored key grew: {fee:?}");

        let key = stored_key(&drive, identity_id);
        assert_eq!(key.total_budget(), Some(RAISED_BUDGET));
        assert_eq!(
            key.expires_at(),
            Some(EXPIRES_AT),
            "the expiry is untouched"
        );
        assert_eq!(
            remaining_budget(&drive, identity_id),
            Some(700_000 + (RAISED_BUDGET - BUDGET))
        );
    }

    #[test]
    fn should_move_the_expiry_later_and_leave_the_budget_alone() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(Some(BUDGET), Some(EXPIRES_AT));

        drive
            .update_identity_key_limits(
                identity_id,
                KEY_ID,
                None,
                Some(EXPIRES_AT + 1_000_000),
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to extend the expiry");

        let key = stored_key(&drive, identity_id);
        assert_eq!(key.expires_at(), Some(EXPIRES_AT + 1_000_000));
        assert_eq!(key.total_budget(), Some(BUDGET));
        assert_eq!(remaining_budget(&drive, identity_id), Some(BUDGET));
    }

    #[test]
    fn should_estimate_at_least_what_the_rewrite_costs() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(Some(BUDGET), None);

        let estimated = drive
            .update_identity_key_limits(
                identity_id,
                KEY_ID,
                Some(RAISED_BUDGET),
                None,
                &block(),
                false,
                None,
                platform_version,
            )
            .expect("expected to estimate");
        let actual = drive
            .update_identity_key_limits(
                identity_id,
                KEY_ID,
                Some(RAISED_BUDGET),
                None,
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to apply");

        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "estimated storage {} below actual {}",
            estimated.storage_fee,
            actual.storage_fee
        );
        assert!(
            estimated.processing_fee >= actual.processing_fee,
            "estimated processing {} below actual {}",
            estimated.processing_fee,
            actual.processing_fee
        );
    }

    /// The key reference trees hold the value hash of the key they point at, so the whole
    /// database only stays consistent when the references are refreshed along with the
    /// rewritten key.
    #[test]
    fn should_refresh_the_references_to_the_rewritten_key() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(Some(BUDGET), Some(EXPIRES_AT));

        drive
            .update_identity_key_limits(
                identity_id,
                KEY_ID,
                Some(RAISED_BUDGET),
                Some(EXPIRES_AT + 1),
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to raise the limits");

        let mismatches = drive
            .grove
            .visualize_verify_grovedb(None, true, true, &platform_version.drive.grove_version)
            .expect("expected to verify the database");
        assert!(
            mismatches.is_empty(),
            "every reference must carry the hash of the rewritten key: {mismatches:?}"
        );
        assert_eq!(
            stored_key(&drive, identity_id).total_budget(),
            Some(RAISED_BUDGET)
        );
    }

    #[test]
    fn should_add_to_the_remaining_budget_of_a_budgeted_key_only() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(Some(BUDGET), None);

        let (operations, remaining) = drive
            .add_to_identity_key_budget_operations(identity_id, KEY_ID, 5, None, platform_version)
            .expect("expected the addition");
        assert_eq!(remaining, BUDGET + 5);
        assert!(!operations.is_empty());

        let (operations, remaining) = drive
            .add_to_identity_key_budget_operations(identity_id, KEY_ID, 0, None, platform_version)
            .expect("expected a no-op");
        assert_eq!(remaining, BUDGET);
        assert!(
            operations.iter().all(|operation| !matches!(
                operation,
                crate::fees::op::LowLevelDriveOperation::GroveOperation(_)
            )),
            "adding nothing writes nothing"
        );

        // Key 0 has no budget entry
        let result =
            drive.add_to_identity_key_budget_operations(identity_id, 0, 5, None, platform_version);
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "{result:?}"
        );
    }

    #[test]
    fn should_not_be_active_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let (drive, identity_id) = setup(Some(BUDGET), None);

        let result = drive.update_identity_key_limits(
            identity_id,
            KEY_ID,
            Some(RAISED_BUDGET),
            None,
            &block(),
            true,
            None,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::VersionNotActive { .. }))
            ),
            "{result:?}"
        );
    }
}
