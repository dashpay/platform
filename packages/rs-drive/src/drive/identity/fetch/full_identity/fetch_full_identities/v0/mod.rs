use crate::drive::Drive;

use crate::error::Error;

use dpp::identity::Identity;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches identities with all its information from storage.
    pub(super) fn fetch_full_identities_v0(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<Identity>>, Error> {
        identity_ids
            .iter()
            .map(|identity_id| {
                Ok((
                    *identity_id,
                    self.fetch_full_identity(*identity_id, transaction, platform_version)?,
                ))
            })
            .collect()
    }
}
