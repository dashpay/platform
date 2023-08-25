use dpp::data_contract::DataContract;
use std::sync::Arc;

use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::ProtocolError;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
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
        // TODO: We should return Option<Arc<DataContractFetchInfo>> but DPP doesn't know about it
        //  or use clone which is not great
        let get_data_contract =
            |data_contract_id: Identifier| -> Result<Option<&DataContract>, ProtocolError> {
                let (_, maybe_contract_with_fetch_info) = drive
                    .get_contract_with_fetch_info_and_fee(
                        data_contract_id.to_buffer(),
                        None,
                        true,
                        tx,
                        platform_version,
                    ) // TODO: Create a special error
                    .map_err(|e| {
                        ProtocolError::Generic(format!("fetch data contract from cache error: {e}"))
                    })?;

                if let Some(ref contract_with_fetch_info) = maybe_contract_with_fetch_info {
                    Ok(Some(&contract_with_fetch_info.contract))
                } else {
                    Ok(None)
                }
            };

        self.validate(get_data_contract, platform_version)
            .map_err(|e| e.into())
    }
}
