use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::Purpose;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves identities keys bound to specified contract
    #[inline(always)]
    pub(super) fn prove_identities_contract_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let query = Self::identities_contract_keys_query(
            identity_ids,
            contract_id,
            &document_type_name,
            &purposes,
            Some((identity_ids.len() * purposes.len()) as u16),
        );
        self.grove_get_proved_path_query(
            &query,
            false,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }
}
