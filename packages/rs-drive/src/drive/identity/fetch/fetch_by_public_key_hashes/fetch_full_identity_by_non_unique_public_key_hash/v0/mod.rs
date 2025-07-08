use crate::drive::Drive;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::identity::Identity;
use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its information from storage.
    pub(super) fn fetch_full_identity_by_non_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identity_by_non_unique_public_key_hash_operations_v0(
            public_key_hash,
            after,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_full_identity_by_non_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let identity_ids = self.fetch_identity_ids_by_non_unique_public_key_hash_operations(
            public_key_hash,
            Some(1),
            after,
            transaction,
            drive_operations,
            platform_version,
        )?;
        if let Some(identity_id) = identity_ids.first() {
            self.fetch_full_identity(*identity_id, transaction, platform_version)
        } else {
            Ok(None)
        }
    }
}
