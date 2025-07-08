use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds a identity by inserting a new identity subtree structure to the `Identities` subtree.
    pub fn add_new_identity(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_v0(
                identity,
                is_masternode_identity,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds identity creation operations to drive operations
    #[allow(clippy::too_many_arguments)]
    pub fn add_new_identity_add_to_operations(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_add_to_operations_v0(
                identity,
                is_masternode_identity,
                block_info,
                apply,
                previous_batch_operations,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// The operations needed to create an identity
    pub fn add_new_identity_operations(
        &self,
        identity: Identity,
        is_masternode_identity: bool,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
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
            .insert
            .add_new_identity
        {
            0 => self.add_new_identity_operations_v0(
                identity,
                is_masternode_identity,
                block_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::identity::Identity;

    use dpp::block::block_info::BlockInfo;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::accessors::IdentityGettersV0;

    use dpp::version::PlatformVersion;

    #[test]
    fn test_insert_and_fetch_identity_first_version() {
        let platform_version = PlatformVersion::first();
        let expected_fee_result = FeeResult {
            storage_fee: 128871000,
            processing_fee: 2330320,
            ..Default::default()
        };

        test_insert_and_fetch_identity(true, platform_version, expected_fee_result);
    }

    #[test]
    fn test_insert_and_fetch_identity_latest_version() {
        let platform_version = PlatformVersion::latest();
        let expected_fee_result = FeeResult {
            storage_fee: 128871000,
            // 2 extra loaded bytes because the token tree is no longer empty
            // these 2 loaded bytes cost 20 credits each
            processing_fee: 2330360,
            ..Default::default()
        };

        test_insert_and_fetch_identity(true, platform_version, expected_fee_result);
    }

    #[test]
    fn test_insert_identity_estimated_costs_first_version() {
        let platform_version = PlatformVersion::first();
        let expected_fee_result = FeeResult {
            storage_fee: 128871000,
            processing_fee: 11764980,
            ..Default::default()
        };

        test_insert_and_fetch_identity(false, platform_version, expected_fee_result);
    }

    #[test]
    fn test_insert_identity_estimated_costs_latest_version() {
        let platform_version = PlatformVersion::latest();
        let expected_fee_result = FeeResult {
            storage_fee: 128871000,
            processing_fee: 11764980,
            ..Default::default()
        };

        test_insert_and_fetch_identity(false, platform_version, expected_fee_result);
    }

    fn test_insert_and_fetch_identity(
        apply: bool,
        platform_version: &PlatformVersion,
        expected_fee_result: FeeResult,
    ) {
        let drive = setup_drive(None);

        let transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        let fee_result = drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                apply,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        if apply {
            let fetched_identity = drive
                .fetch_full_identity(
                    identity.id().to_buffer(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("should fetch an identity")
                .expect("should have an identity");
            assert_eq!(identity, fetched_identity);
        }
        assert_eq!(fee_result, expected_fee_result);
    }
}
