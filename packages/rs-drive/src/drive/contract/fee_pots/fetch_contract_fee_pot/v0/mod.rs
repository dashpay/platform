use crate::drive::contract::fee_pots::types::{decode_last_claim, ContractFeePotState};
use crate::drive::contract::paths::{
    contract_fee_pots_path, contract_last_fee_claim_key, contract_other_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_fee_pot_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContractFeePotState, Error> {
        let pots_path = contract_fee_pots_path(pot);
        let credits = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&pots_path).into(),
                contract_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or_default();

        let other_path = contract_other_path(contract_id.as_slice());
        let last_claim = self
            .grove_get_raw_optional_item(
                (&other_path).into(),
                contract_last_fee_claim_key(pot),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            .map(|value| {
                decode_last_claim(&value).map_err(|description| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "last claim of the {} fee pot of contract {} is malformed: {}",
                        pot, contract_id, description
                    )))
                })
            })
            .transpose()?;

        Ok(ContractFeePotState {
            credits,
            last_claim,
        })
    }
}
