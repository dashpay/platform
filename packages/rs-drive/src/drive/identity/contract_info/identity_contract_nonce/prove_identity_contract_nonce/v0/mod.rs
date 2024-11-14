use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Proves the Identity's contract nonce from the backing store
    pub(super) fn prove_identity_contract_nonce_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity_contract_path = Self::identity_contract_nonce_query(identity_id, contract_id);
        self.grove_get_proved_path_query(
            &identity_contract_path,
            transaction,
            &mut vec![],
            drive_version,
        )
    }
}
