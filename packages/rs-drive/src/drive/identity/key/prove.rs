use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence of all keys associated with the specified identities.
    ///
    /// This function creates a path query for each identity ID provided, requesting
    /// all keys associated with each identity. It then proves the existence of the keys
    /// using the provided `transaction`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - A slice of identity IDs as 32-byte arrays. Each identity ID is used to
    ///   create a path query for proving its associated keys.
    /// * `limit` - An optional `u16` value specifying the maximum number of keys to fetch for each
    ///   identity. If `None`, fetches all available keys.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for proving the existence of the keys.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - If successful, returns a `Vec<u8>` containing the proof data.
    ///   If an error occurs during the proof generation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the path query generation or proof generation fails.

    pub fn prove_identities_all_keys(
        &self,
        identity_ids: &[[u8; 32]],
        limit: Option<u16>,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity_query = Self::fetch_identities_all_keys_query(&self, identity_ids, limit)?;
        self.grove_get_proved_path_query(&identity_query, false, transaction, &mut vec![])
    }

    /// Prove the requested identity keys.
    ///
    /// This function takes an `IdentityKeysRequest` and a `TransactionArg` as arguments
    /// and returns a proof of the requested identity keys as a `Vec<u8>` or an error
    /// if the proof cannot be generated.
    ///
    /// # Arguments
    ///
    /// * `key_request` - An `IdentityKeysRequest` containing the details of the
    ///   requested identity keys, such as the identity ID, request type, limit, and offset.
    /// * `transaction` - A `TransactionArg` representing the current transaction.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - A proof of the requested identity keys as a `Vec<u8>` if the
    ///   proof is successfully generated.
    /// * `Err(Error)` - An error if the proof cannot be generated.
    ///
    pub fn prove_identity_keys(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let identity_query = key_request.into_path_query();
        self.grove_get_proved_path_query(&identity_query, false, transaction, &mut vec![])
    }
}
