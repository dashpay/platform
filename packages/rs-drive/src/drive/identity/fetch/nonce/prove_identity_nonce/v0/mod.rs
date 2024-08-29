use crate::drive::Drive;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the Identity's nonce from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn prove_identity_nonce_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let nonce_query = Self::identity_nonce_query(identity_id);
        self.grove_get_proved_path_query(
            &nonce_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
