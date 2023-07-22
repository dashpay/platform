use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identities(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .full_identity
            .fetch_full_identities
        {
            Some(0) => self.fetch_full_identities_v0(identity_ids, transaction, platform_version),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identities".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_full_identities".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
