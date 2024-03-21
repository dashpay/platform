use crate::drive::Drive;
use crate::error::Error;
use dpp::identity::{IdentityPublicKey, KeyID};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches all keys associated with the specified identities.
    ///
    /// This function retrieves all keys associated with each identity ID provided
    /// and returns the result as a `BTreeMap` mapping the identity IDs to their respective keys.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - A slice of identity IDs as 32-byte arrays. Each identity ID is used to
    ///   fetch its associated keys.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for fetching the keys.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Vec<BTreeMap<KeyID, IdentityPublicKey>>>, Error>` - If successful,
    ///   returns a `BTreeMap` where the keys are the identity IDs and the values are `Vec`s containing
    ///   `BTreeMap`s mapping `KeyID`s to `IdentityPublicKey`s. If an error occurs during the key
    ///   fetching, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the key fetching fails.
    #[inline(always)]
    pub(super) fn fetch_identities_all_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], BTreeMap<KeyID, IdentityPublicKey>>, Error> {
        identity_ids
            .iter()
            .map(|identity_id| {
                Ok((
                    *identity_id,
                    Self::fetch_all_identity_keys(
                        self,
                        *identity_id,
                        transaction,
                        platform_version,
                    )?,
                ))
            })
            .collect()
    }
}
