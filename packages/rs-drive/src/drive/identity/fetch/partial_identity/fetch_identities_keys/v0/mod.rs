use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap,
    KeyIDOptionalIdentityPublicKeyPairBTreeMap,
};
use crate::drive::Drive;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::fee::default_costs::KnownCostItem::FetchIdentityBalanceProcessingCost;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, PartialIdentity};
use grovedb::TransactionArg;

use dpp::fee::default_costs::EpochCosts;

use dpp::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identities_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<PartialIdentity>>, Error> {
        identity_ids
            .iter()
            .map(|identity_id| {
                let keys_request = IdentityKeysRequest::new_all_keys_query(
                    identity_id,
                    None, // TODO: set some limit?
                );

                let public_keys_with_optionals = self
                    .fetch_identity_keys::<BTreeMap::<[u8; 32], Vec<IdentityPublicKey>>>(
                        keys_request,
                        transaction,
                        platform_version,
                    )?;

                let mut loaded_public_keys = BTreeMap::new();
                let mut not_found_public_keys = BTreeSet::new();

                public_keys_with_optionals
                    .into_iter()
                    .for_each(|(keyId, key)| {
                        match key {
                            None => {
                                not_found_public_keys.insert(keyId);
                            }
                            Some(key) => {
                                loaded_public_keys.insert(keyId, key);
                            }
                        }
                    });

                let identity = PartialIdentity {
                    id: Identifier::new(*identity_id),
                    loaded_public_keys,
                    balance: None,
                    revision: None,
                    not_found_public_keys,
                };

                Ok((
                    *identity_id,
                    Some(identity)
                ))
            })
            .collect()
    }
}
