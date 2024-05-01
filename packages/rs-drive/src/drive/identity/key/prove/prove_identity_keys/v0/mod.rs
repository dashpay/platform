use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
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
    #[inline(always)]
    pub(super) fn prove_identity_keys_v0(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity_query = key_request.into_path_query();
        self.grove_get_proved_path_query(
            &identity_query,
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
