mod v0;

use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of the contract groups a contract belongs to, as a whole, through its
    /// document types and through its tokens. Empty when the contract belongs to no group.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_contract_group_memberships_for_contract` produced it.
    /// * `contract_id`: The contract's id.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, ContractGroupMembershipsForContract))` with the proof's root hash and
    ///   the groups of the whole contract, of each document type and of each token; empty when
    ///   it belongs to no group.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or a
    ///   proven membership is malformed.
    pub fn verify_contract_group_memberships_for_contract(
        proof: &[u8],
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractGroupMembershipsForContract), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_group
            .verify_contract_group_memberships_for_contract
        {
            0 => Self::verify_contract_group_memberships_for_contract_v0(
                proof,
                contract_id,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_group_memberships_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
