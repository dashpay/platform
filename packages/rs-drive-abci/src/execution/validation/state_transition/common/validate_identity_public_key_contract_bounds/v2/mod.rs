use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::identity::{
    InvalidIdentityPublicKeySecurityLevelError, InvalidKeyPurposeForContractBoundsError,
};
use dpp::consensus::state::contract_group::ContractGroupNotFoundError;
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
/// rules unchanged. A contract group bound is admitted on AUTHENTICATION keys only: the group
/// must exist (one billed read), and a group has no config to opt encryption or decryption keys
/// in with, so those purposes are refused.
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
        if let Some(contract_group_id) = bounds.contract_group_id() {
            if key.purpose() != Purpose::AUTHENTICATION {
                result.add_error(InvalidKeyPurposeForContractBoundsError::new(
                    key.purpose(),
                    vec![Purpose::AUTHENTICATION],
                ));
                continue;
            }
            if key.security_level() == SecurityLevel::MASTER {
                result.add_error(master_key_cannot_be_bound_error(key));
                continue;
            }
            let (fee, info) = drive.fetch_contract_group_info_with_fee(
                *contract_group_id,
                epoch,
                transaction,
                version,
            )?;
            context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            if info.is_none() {
                result.add_error(ContractGroupNotFoundError::new(*contract_group_id));
            }
            continue;
        }
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
            result.add_error(master_key_cannot_be_bound_error(key));
            continue;
        }
        let Some(contract_id) = bounds.contract_id().copied() else {
            // Contract group bounds were handled above.
            continue;
        };
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

fn master_key_cannot_be_bound_error(
    key: &IdentityPublicKeyInCreation,
) -> InvalidIdentityPublicKeySecurityLevelError {
    InvalidIdentityPublicKeySecurityLevelError::new(
        key.id(),
        key.purpose(),
        key.security_level(),
        Some(vec![
            SecurityLevel::CRITICAL,
            SecurityLevel::HIGH,
            SecurityLevel::MEDIUM,
        ]),
    )
}
