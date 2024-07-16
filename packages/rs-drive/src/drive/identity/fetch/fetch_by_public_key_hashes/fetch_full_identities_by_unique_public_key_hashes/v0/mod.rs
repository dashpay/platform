use crate::drive::Drive;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::identity::Identity;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches identities with all its information from storage.
    pub(super) fn fetch_full_identities_by_unique_public_key_hashes_v0(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 20], Option<Identity>>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_full_identities_by_unique_public_key_hashes_operations_v0(
            public_key_hashes,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub(super) fn fetch_full_identities_by_unique_public_key_hashes_operations_v0(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 20], Option<Identity>>, Error> {
        let identity_ids = self.fetch_identity_ids_by_unique_public_key_hashes_operations(
            public_key_hashes,
            transaction,
            drive_operations,
            platform_version,
        )?;
        identity_ids
            .into_iter()
            .map(|(public_key_hash, maybe_identity_id)| {
                let identity = maybe_identity_id
                    .map(|identity_id| {
                        self.fetch_full_identity(identity_id, transaction, platform_version)
                    })
                    .transpose()?
                    .flatten();
                Ok((public_key_hash, identity))
            })
            .collect::<Result<BTreeMap<[u8; 20], Option<Identity>>, Error>>()
    }
}
