use std::sync::Arc;
use dpp::consensus::ConsensusError;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::grovedb::Transaction;
use platform_version::version::PlatformVersion;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

pub(crate) fn validate_identity_public_keys_contract_bounds_v0(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    transaction: &Transaction,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    //todo: we should add to the execution context the cost of fetching contracts
    for identity_public_key in identity_public_keys_with_witness {
        let purpose = identity_public_key.purpose();
        if let Some(contract_bounds) = identity_public_key.contract_bounds() {
            match contract_bounds {
                ContractBounds::SingleContract { id : contract_id } => {
                    // we should fetch the contract
                    let contract = drive.fetch_contract(contract_id.to_buffer(), None, None, Some(transaction), platform_version).unwrap()?;
                    match contract {
                        None => { return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError())) }
                        Some(contract) => {
                            if contract.contract
                        }
                    }
                }
                ContractBounds::SingleContractDocumentType { id : contract_id, document_type: String } => {

                }
                ContractBounds::MultipleContractsOfSameOwner { owner_id } => {

                }
            }
        }
    }
}