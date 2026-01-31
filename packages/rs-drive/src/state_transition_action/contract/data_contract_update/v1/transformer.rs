use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::contract::data_contract_update::v1::DataContractUpdateTransitionActionV1;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::errors::DataContractNotPresentError;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::validation::ConsensusValidationResult;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionActionV1 {
    /// Transforms a V1 update transition into an action by fetching the old contract
    /// from Drive and applying the updates.
    pub(in crate::state_transition_action::contract::data_contract_update) fn try_from_transition(
        value: &DataContractUpdateTransitionV1,
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        // Fetch the old contract from Drive
        let old_data_contract = drive
            .get_contract_with_fetch_info_and_fee(
                value.id.to_buffer(),
                None,
                false,
                transaction,
                platform_version,
            )?
            .1
            .ok_or_else(|| {
                Error::Protocol(Box::new(ProtocolError::DataContractNotPresentError(
                    DataContractNotPresentError::new(value.id),
                )))
            })?
            .contract
            .clone();

        // Build the new contract by applying updates to the old contract
        let validation_result = old_data_contract
            .apply_update(
                value.into(),
                block_info,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map_err(|e| Error::Protocol(Box::new(e)))?;

        Ok(
            validation_result.map(|new_data_contract| DataContractUpdateTransitionActionV1 {
                old_data_contract,
                data_contract: new_data_contract,
                identity_contract_nonce: value.identity_contract_nonce,
                user_fee_increase: value.user_fee_increase,
            }),
        )
    }
}
