use crate::drive::Drive;
use crate::error::Error;

use dpp::identity::KeyID;
use grovedb::TransactionArg;

use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use dpp::prelude::IdentityPublicKey;
use dpp::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identities_keys_v0(
        &self,
        identities_key_ids: &HashMap<[u8; 32], Vec<u32>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], BTreeMap<KeyID, IdentityPublicKey>>, Error> {
        identities_key_ids
            .iter()
            .map(|(identity_id, key_ids)| {
                let key_ids = key_ids.into_iter().map(|id| *id as KeyID).collect();

                // TODO: put an upper bound on the number of keys that can be fetched?
                let key_request = IdentityKeysRequest {
                    identity_id: *identity_id,
                    request_type: KeyRequestType::SpecificKeys(key_ids),
                    limit: None,
                    offset: None,
                };

                let public_keys =
                    self.fetch_identity_keys(key_request, transaction, platform_version)?;

                Ok((*identity_id, public_keys))
            })
            .collect()
    }
}
