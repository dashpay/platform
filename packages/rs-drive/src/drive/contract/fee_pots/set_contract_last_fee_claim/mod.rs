mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// The operations that record the claim that paid out one of a contract's fee pots: its
    /// epoch, its block time and who claimed. A pot is claimed at most once per epoch, and the
    /// epoch of this item is what the next claim is judged against.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pot belongs to.
    /// * `pot`: The pot.
    /// * `last_claim`: The claim.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the write.
    /// * `Err(Error)` when the version is unknown.
    pub fn set_contract_last_fee_claim_operations(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        last_claim: &ContractFeePotLastClaim,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .set_contract_last_fee_claim
        {
            0 => self.set_contract_last_fee_claim_operations_v0(
                contract_id,
                pot,
                last_claim,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_contract_last_fee_claim_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
