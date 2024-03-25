use crate::drive::Drive;
use crate::error::Error;

use dpp::identity::{KeyID};
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;
use std::collections::{BTreeMap};
use dpp::prelude::IdentityPublicKey;

impl Drive {
    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identities_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], BTreeMap<KeyID, IdentityPublicKey>>, Error> {
        identity_ids
            .iter()
            .map(|identity_id| {
                let public_keys = self
                    .fetch_all_identity_keys(
                        *identity_id,
                        transaction,
                        platform_version
                    )?;

                Ok((*identity_id, public_keys))
            })
            .collect()
    }
}
