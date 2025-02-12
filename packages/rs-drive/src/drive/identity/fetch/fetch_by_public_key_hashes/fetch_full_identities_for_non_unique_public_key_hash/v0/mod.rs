use crate::drive::Drive;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::identity::Identity;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Fetches identities with all its information from storage.
    pub(super) fn fetch_full_identities_for_non_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identities_for_non_unique_public_key_hash_operations_v0(
            public_key_hash,
            limit,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_full_identities_for_non_unique_public_key_hash_operations_v0(
        &self,
        public_key_hash: [u8; 20],
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identity>, Error> {
        let identity_ids = self.fetch_identity_ids_by_non_unique_public_key_hash_operations_v0(
            public_key_hash,
            limit,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        identity_ids
            .into_iter()
            .filter_map(|identity_id| {
                self.fetch_full_identity(identity_id, transaction, platform_version)
                    .transpose()
            })
            .collect::<Result<Vec<Identity>, Error>>()
    }
}
