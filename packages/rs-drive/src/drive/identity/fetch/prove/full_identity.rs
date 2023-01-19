use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an identity with all its information from an identity id.
    pub fn prove_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identity_query(&identity_id)?;
        self.grove_get_proved_path_query(&query, transaction, &mut drive_operations)
    }

    /// Proves identities with all its information from an identity ids.
    pub fn proved_full_identities(
        &self,
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let query = Self::full_identities_query(identity_ids)?;
        self.grove_get_proved_path_query(&query, transaction, &mut drive_operations)
    }
}
