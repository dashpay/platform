use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDOptionalIdentityPublicKeyPairBTreeMap,
};
use crate::drive::Drive;
use crate::error::Error;

use dpp::identifier::Identifier;
use dpp::identity::PartialIdentity;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_balance_with_keys_and_revision_v0(
        &self,
        identity_key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PartialIdentity>, Error> {
        let id = Identifier::new(identity_key_request.identity_id);
        let balance = self.fetch_identity_balance(
            identity_key_request.identity_id,
            transaction,
            platform_version,
        )?;
        let Some(balance) = balance else {
            return Ok(None);
        };

        //todo: deal with apply
        let revision = self.fetch_identity_revision(
            identity_key_request.identity_id,
            true,
            transaction,
            platform_version,
        )?;
        let Some(revision) = revision else {
            return Ok(None);
        };

        let public_keys_with_optionals = self
            .fetch_identity_keys::<KeyIDOptionalIdentityPublicKeyPairBTreeMap>(
                identity_key_request,
                transaction,
                platform_version,
            )?;

        let mut loaded_public_keys = BTreeMap::new();
        let mut not_found_public_keys = BTreeSet::new();

        public_keys_with_optionals
            .into_iter()
            .for_each(|(key, value)| {
                match value {
                    None => {
                        not_found_public_keys.insert(key);
                    }
                    Some(value) => {
                        loaded_public_keys.insert(key, value);
                    }
                };
            });
        Ok(Some(PartialIdentity {
            id,
            loaded_public_keys,
            balance: Some(balance),
            revision: Some(revision),
            contract_revisions: Default::default(),
            not_found_public_keys,
        }))
    }
}
