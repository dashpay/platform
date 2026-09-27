mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::contract_group::ContractGroupInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of a contract group's stored information: owner, name and description.
    ///
    /// Returns the root hash and the information, or `None` when the proof shows the group is
    /// absent.
    pub fn verify_contract_group_info(
        proof: &[u8],
        contract_group_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<ContractGroupInfo>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_group
            .verify_contract_group_info
        {
            0 => Self::verify_contract_group_info_v0(proof, contract_group_id, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_group_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
