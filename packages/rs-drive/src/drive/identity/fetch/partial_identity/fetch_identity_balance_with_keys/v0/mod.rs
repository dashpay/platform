use crate::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap,
    KeyIDOptionalIdentityPublicKeyPairBTreeMap,
};
use crate::drive::Drive;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::PartialIdentity;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl Drive {
    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_balance_with_keys_v0(
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
            revision: None,

            not_found_public_keys,
        }))
    }

    /// Fetches the Identity's balance with keys as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_balance_with_keys_with_cost_v0(
        &self,
        identity_key_request: IdentityKeysRequest,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<PartialIdentity>, FeeResult), Error> {
        let balance_cost = platform_version
            .fee_version
            .processing
            .fetch_identity_balance_processing_cost;
        if !apply {
            let keys_cost = identity_key_request.processing_cost(platform_version)?;
            return Ok((
                None,
                FeeResult::new_from_processing_fee(balance_cost + keys_cost),
            ));
        }
        // let's start by getting the balance
        let id = Identifier::new(identity_key_request.identity_id);
        let balance = self.fetch_identity_balance(
            identity_key_request.identity_id,
            transaction,
            platform_version,
        )?;
        let Some(balance) = balance else {
            return Ok((None, FeeResult::new_from_processing_fee(balance_cost)));
        };

        let keys_cost = identity_key_request.processing_cost(platform_version)?;

        let loaded_public_keys = self.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
            identity_key_request,
            transaction,
            platform_version,
        )?;
        Ok((
            Some(PartialIdentity {
                id,
                loaded_public_keys,
                balance: Some(balance),
                revision: None,

                not_found_public_keys: Default::default(),
            }),
            FeeResult::new_from_processing_fee(balance_cost + keys_cost),
        ))
    }
}
