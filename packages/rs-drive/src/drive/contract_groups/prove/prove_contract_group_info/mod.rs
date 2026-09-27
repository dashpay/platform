mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves a contract group's stored information (owner, name, description), or its absence.
    pub fn prove_contract_group_info(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .prove
            .prove_contract_group_info
        {
            0 => {
                self.prove_contract_group_info_v0(contract_group_id, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_group_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
