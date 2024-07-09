mod v0;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's keys as `PartialIdentityInfo` from the backing store.
    ///
    /// This method selects the appropriate version of the function to call based on the
    /// provided platform version.
    ///
    /// # Parameters
    ///
    /// - `identity_key_request`: A request containing information about the identity whose balance and keys need to be fetched.
    /// - `transaction`: A transaction argument for the database.
    /// - `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option<PartialIdentity>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform version is not recognized or if there's a failure
    /// during the operation.
    pub fn fetch_identity_keys_as_partial_identity(
        &self,
        identity_key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PartialIdentity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .partial_identity
            .fetch_identity_keys
        {
            0 => self.fetch_identity_keys_as_partial_identity_v0(
                identity_key_request,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_keys_as_partial_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
