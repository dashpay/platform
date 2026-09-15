use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::{
    document::{DataContractNotPresentError, InvalidDocumentTypeError},
    identity::InvalidAuthenticationScopeError,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::identity::{contract_bounds::ContractBounds, Purpose, SecurityLevel};
use dpp::state_transition::public_key_in_creation::{
    accessors::IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreation,
};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::{drive::Drive, grovedb::TransactionArg};

/// v2 adds authentication delegations; encryption/decryption use unchanged v1 rules.
#[allow(clippy::too_many_arguments)] // Keep explicit versioned validation inputs.
pub(super) fn validate_identity_public_keys_contract_bounds_v2(
    identity_id: Identifier,
    keys: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    epoch: &Epoch,
    time_ms: u64,
    transaction: TransactionArg,
    context: &mut StateTransitionExecutionContext,
    version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();
    for key in keys {
        let Some(ContractBounds::Scoped(scope)) = key.contract_bounds() else {
            result.add_errors(
                super::v1::validate_identity_public_keys_contract_bounds_v1(
                    identity_id,
                    std::slice::from_ref(key),
                    drive,
                    epoch,
                    transaction,
                    context,
                    version,
                )?
                .errors,
            );
            continue;
        };
        if key.purpose() != Purpose::AUTHENTICATION
            || key.security_level() == SecurityLevel::MASTER
            || scope.validate().is_err()
            || scope.is_expired(time_ms)
        {
            result.add_error(InvalidAuthenticationScopeError::new(
                "scope must be valid, unexpired and attached to a non-MASTER authentication key"
                    .into(),
            ));
            continue;
        }
        for entry in scope.contracts() {
            let outcome = drive.get_system_or_user_contract_with_fee(
                entry.id.to_buffer(),
                epoch,
                transaction,
                version,
            )?;
            if let Some(fee) = outcome.fee() {
                context.add_operation(ValidationOperation::PrecalculatedOperation(fee.clone()));
            }
            let Some(contract) = outcome.contract() else {
                result.add_error(DataContractNotPresentError::new(entry.id));
                continue;
            };
            if let Some(names) = &entry.document_types {
                for name in names {
                    if contract.document_type_optional_for_name(name).is_none() {
                        result.add_error(InvalidDocumentTypeError::new(name.clone(), entry.id));
                    }
                }
            }
        }
    }
    Ok(result)
}
