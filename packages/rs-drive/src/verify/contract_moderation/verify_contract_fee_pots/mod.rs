mod v0;

use crate::drive::contract::fee_pots::types::ContractFeePots;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies the proof of a contract's fee pots: the credits each proved pot holds and the
    /// last claim: its epoch, its block time and who claimed.
    ///
    /// Only the pots in `pots` are proved. The other one is returned empty and never claimed,
    /// which says nothing about it: callers that did not ask for a pot must not read it.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof.
    /// * `contract_id`: The contract the pots belong to.
    /// * `pots`: The pots that were proved, at least one.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than these pots, as the
    ///   proof of a contract fee claim's execution does.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, ContractFeePots))` with the root hash and the pots.
    /// * `Err(Error)` when the version is unknown or the proof is not one of these pots.
    pub fn verify_contract_fee_pots(
        proof: &[u8],
        contract_id: Identifier,
        pots: &[ContractFeePot],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractFeePots), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_fee_pots
        {
            0 => Self::verify_contract_fee_pots_v0(
                proof,
                contract_id,
                pots,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_fee_pots".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
