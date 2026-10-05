use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::balances::credits::Credits;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on the fee pots a contract's document action fees accumulate in.
///
/// Neither `AddToPot` nor `DeductFromPot` creates or destroys credits: the batch that carries
/// one also moves the same credits out of, or into, identity balances.
#[derive(Clone, Debug)]
pub enum ContractFeePotOperationType {
    /// Adds credits to a pot. The new total is computed from the committed one, so
    /// `apply_drive_operations` merges every write of one pot in a batch into one.
    AddToPot {
        /// The contract the pot belongs to.
        contract_id: Identifier,
        /// The pot.
        pot: ContractFeePot,
        /// The credits to add.
        amount: Credits,
    },
    /// Takes credits out of a pot.
    DeductFromPot {
        /// The contract the pot belongs to.
        contract_id: Identifier,
        /// The pot.
        pot: ContractFeePot,
        /// The credits to take out.
        amount: Credits,
    },
    /// Records the claim that paid a pot out.
    SetLastClaim {
        /// The contract the pot belongs to.
        contract_id: Identifier,
        /// The pot.
        pot: ContractFeePot,
        /// The claim: its epoch, its block time and who claimed.
        last_claim: ContractFeePotLastClaim,
    },
}

impl DriveLowLevelOperationConverter for ContractFeePotOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            ContractFeePotOperationType::AddToPot {
                contract_id,
                pot,
                amount,
            } => drive.add_to_contract_fee_pot_operations(
                contract_id,
                pot,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractFeePotOperationType::DeductFromPot {
                contract_id,
                pot,
                amount,
            } => drive.deduct_from_contract_fee_pot_operations(
                contract_id,
                pot,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractFeePotOperationType::SetLastClaim {
                contract_id,
                pot,
                last_claim,
            } => drive.set_contract_last_fee_claim_operations(
                contract_id,
                pot,
                &last_claim,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
        }
    }
}
