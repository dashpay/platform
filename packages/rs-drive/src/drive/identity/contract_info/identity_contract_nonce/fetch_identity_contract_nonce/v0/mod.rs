use crate::drive::identity::identity_contract_info_group_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::IdentityNonce;

use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// Fetches the Identity's contract revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_contract_nonce_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityNonce>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_contract_nonce_operations_v0(
            identity_id,
            contract_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Creates the operations to get Identity's contract revision from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(super) fn fetch_identity_contract_nonce_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityNonce>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(1),
            }
        };
        let identity_contract_path =
            identity_contract_info_group_path(&identity_id, contract_id.as_slice());
        match self.grove_get_raw_optional(
            (&identity_contract_path).into(),
            &[IdentityContractNonceKey as u8],
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(encoded_nonce, _))) => {
                let nonce =
                    IdentityNonce::from_be_bytes(encoded_nonce.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "identity contract nonce was not 8 bytes as expected",
                        ))
                    })?);

                Ok(Some(nonce))
            }

            Ok(None) => Ok(None),

            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity contract nonce was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// Fetches the Identity's contract revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_contract_nonce_with_fees_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<IdentityNonce>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_contract_nonce_operations_v0(
            identity_id,
            contract_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((value, fees))
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;

    fn new_drive_with_identity() -> (crate::drive::Drive, Identity) {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("init");
        let identity = Identity::random_identity(5, Some(42), platform_version).expect("rand id");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("insert identity");
        (drive, identity)
    }

    /// Fetching a contract nonce for an identity that has never interacted with
    /// the contract returns None without erroring.
    #[test]
    fn fetch_nonce_for_identity_with_no_contract_returns_none() {
        let (drive, identity) = new_drive_with_identity();
        let platform_version = PlatformVersion::first();

        let result = drive
            .fetch_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                [9u8; 32],
                true,
                None,
                platform_version,
            )
            .expect("fetch should return Ok(None)");
        assert_eq!(result, None);
    }

    /// Fetching a contract nonce for a non-existent identity returns None.
    #[test]
    fn fetch_nonce_for_nonexistent_identity_returns_none() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("init");

        let result = drive
            .fetch_identity_contract_nonce_v0([0u8; 32], [0u8; 32], true, None, platform_version)
            .expect("fetch for missing identity");
        assert_eq!(result, None);
    }

    /// After merging a nonce, the fetch returns Some(nonce). Exercises the
    /// success round-trip through fetch_identity_contract_nonce_v0.
    #[test]
    fn fetch_after_merge_returns_the_stored_nonce() {
        let (drive, identity) = new_drive_with_identity();
        let platform_version = PlatformVersion::first();

        let contract_id = [3u8; 32];
        drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("merge nonce");

        let fetched = drive
            .fetch_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                true,
                None,
                platform_version,
            )
            .expect("fetch after merge");
        assert!(fetched.is_some());
        // The stored nonce is 1 (the value_filter masks out missing revisions bits).
        let raw = fetched.expect("some");
        assert_eq!(
            raw & dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER,
            1
        );
    }

    /// fetch_identity_contract_nonce_with_fees_v0 returns a FeeResult with
    /// non-zero costs after a real merge happened.
    #[test]
    fn fetch_with_fees_returns_fee_result() {
        let (drive, identity) = new_drive_with_identity();
        let platform_version = PlatformVersion::first();

        let contract_id = [4u8; 32];
        drive
            .merge_identity_contract_nonce_v0(
                identity.id().to_buffer(),
                contract_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("merge");

        let (value, fees) = drive
            .fetch_identity_contract_nonce_with_fees_v0(
                identity.id().to_buffer(),
                contract_id,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("fetch with fees");

        assert!(value.is_some());
        assert!(fees.processing_fee > 0 || fees.storage_fee > 0);
    }

    /// Stateless (apply=false) fetch for a non-existent identity returns None
    /// — this path uses DirectQueryType::StatelessDirectQuery.
    #[test]
    fn stateless_fetch_missing_identity_returns_none() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(None, platform_version)
            .expect("init");

        let mut ops = vec![];
        let result = drive
            .fetch_identity_contract_nonce_operations_v0(
                [0u8; 32],
                [0u8; 32],
                false, // stateless
                None,
                &mut ops,
                platform_version,
            )
            .expect("stateless fetch");
        assert_eq!(result, None);
    }
}
