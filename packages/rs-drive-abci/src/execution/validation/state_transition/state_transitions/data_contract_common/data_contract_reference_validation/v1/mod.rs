use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::data_contract_common::data_contract_reference_validation::v0::validate_data_contract_references_v0;

/// Version 1 is version 0 with the `contract` owner gate admitted.
///
/// A `contract` reference declared with `propertyAgreement: { "$ownerId":
/// "$ownerId" }` carries no declaration content for registration to resolve:
/// a contract has no properties to agree with, and the referenced contract id
/// is a document value, not part of the declaration. The parser already
/// admits exactly that one pair and refuses every other, so at registration
/// the gate needs no state check. Its enforcement lives in document reference
/// validation v1, at write time, where the referenced contract is fetched for
/// the existence check anyway.
///
/// This generation exists because a contract whose parsed references carry
/// the gate is protocol-14 grammar: it is selected only by protocol 14's
/// tables, and delegating to v0 keeps the shipped generation byte-identical.
pub(super) fn validate_data_contract_references_v1(
    contract: &DataContract,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    validate_data_contract_references_v0(
        contract,
        drive,
        block_info,
        execution_context,
        transaction,
        platform_version,
    )
}
