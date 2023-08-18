use dpp::data_contract::DataContract;

use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::ProtocolError;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let get_data_contract =
            |data_contract_id: Identifier| -> Result<Option<&DataContract>, ProtocolError> {
                drive
                    .get_contract_with_fetch_info_and_fee(
                        data_contract_id.0 .0,
                        None,
                        true,
                        tx,
                        platform_version,
                    )
                    .and_then(|(_, maybe_contract_with_fetch_info)| {
                        let maybe_contract = maybe_contract_with_fetch_info
                            .map(|contract_with_fetch_info| &contract_with_fetch_info.contract);

                        Ok(maybe_contract)
                    })
                    // TODO: Create and handle special type of error
                    .map_err(|e| {
                        ProtocolError::Generic(
                            "we should figure out what to do with this error".to_string(),
                        )
                    })
            };

        self.validate(get_data_contract, platform_version)
    }
}
