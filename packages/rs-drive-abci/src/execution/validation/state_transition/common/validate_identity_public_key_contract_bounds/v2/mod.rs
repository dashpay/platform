use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::identity::InvalidIdentityPublicKeySecurityLevelError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::identity::{Purpose, SecurityLevel};
use dpp::state_transition::public_key_in_creation::{
    accessors::IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreation,
};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::{drive::Drive, grovedb::TransactionArg};

/// v2 admits contract bounds on AUTHENTICATION keys: the bound contract (and document type)
/// must exist and the key must not be a MASTER key. Any contract may be bound; there is no
/// contract opt-in or uniqueness rule, unlike encryption and decryption keys, which keep the v1
/// rules unchanged.
#[allow(clippy::too_many_arguments)] // Keep explicit versioned validation inputs.
pub(super) fn validate_identity_public_keys_contract_bounds_v2(
    identity_id: Identifier,
    keys: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    epoch: &Epoch,
    transaction: TransactionArg,
    context: &mut StateTransitionExecutionContext,
    version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();
    for key in keys {
        let Some(bounds) = key.contract_bounds() else {
            continue;
        };
        if key.purpose() != Purpose::AUTHENTICATION {
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
        }
        if key.security_level() == SecurityLevel::MASTER {
            result.add_error(InvalidIdentityPublicKeySecurityLevelError::new(
                key.id(),
                key.purpose(),
                key.security_level(),
                Some(vec![
                    SecurityLevel::CRITICAL,
                    SecurityLevel::HIGH,
                    SecurityLevel::MEDIUM,
                ]),
            ));
            continue;
        }
        let contract_id = *bounds.identifier();
        let outcome = drive.get_system_or_user_contract_with_fee(
            contract_id.to_buffer(),
            epoch,
            transaction,
            version,
        )?;
        if let Some(fee) = outcome.fee() {
            context.add_operation(ValidationOperation::PrecalculatedOperation(fee.clone()));
        }
        let Some(contract) = outcome.contract() else {
            result.add_error(DataContractNotPresentError::new(contract_id));
            continue;
        };
        if let Some(name) = bounds.document_type() {
            if contract.document_type_optional_for_name(name).is_none() {
                result.add_error(InvalidDocumentTypeError::new(name.clone(), contract_id));
            }
        }
    }
    Ok(result)
}
