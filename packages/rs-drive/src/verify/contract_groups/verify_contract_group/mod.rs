mod v0;

use crate::drive::contract_groups::types::ContractGroup;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of a contract group: its information and every member.
    ///
    /// Returns the root hash and the group, or `None` when the proof shows the group is absent.
    pub fn verify_contract_group(
        proof: &[u8],
        contract_group_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<ContractGroup>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_group
            .verify_contract_group
        {
            0 => Self::verify_contract_group_v0(proof, contract_group_id, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_group".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
