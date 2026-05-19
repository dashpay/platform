use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::v0::StateTransitionExecutionContextV0;
use derive_more::From;
use dpp::fee::fee_result::FeeResult;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};

/// V0 module
pub mod v0;

#[derive(Debug, Clone, From)]
/// The state transition execution context
pub enum StateTransitionExecutionContext {
    /// Version 0
    V0(StateTransitionExecutionContextV0),
}

/// The trait defining state transition execution context methods for v0
pub trait StateTransitionExecutionContextMethodsV0 {
    /// Add an operation to the state transition execution context
    fn add_operation(&mut self, operation: ValidationOperation);
    /// Add operations to the state transition execution context
    fn add_operations(&mut self, operations: Vec<ValidationOperation>);
    /// Add dpp operations to the state transition execution context
    fn add_dpp_operations(&mut self, operations: Vec<ProtocolValidationOperation>);
    /// Consume the operations of the context
    fn operations_consume(self) -> Vec<ValidationOperation>;
    /// Returns a slice of operations, does not consume the context
    fn operations_slice(&self) -> &[ValidationOperation];

    /// Are we in a dry run?
    fn in_dry_run(&self) -> bool;
    /// Set us to be in a dry run
    fn enable_dry_run(&mut self);
    /// Set us not to be in a dry run
    fn disable_dry_run(&mut self);

    /// Get the fee costs of all operations in the execution context
    fn fee_cost(&self, platform_version: &PlatformVersion) -> Result<FeeResult, Error>;
}

impl StateTransitionExecutionContextMethodsV0 for StateTransitionExecutionContext {
    fn add_operation(&mut self, operation: ValidationOperation) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.push(operation),
        }
    }

    fn add_operations(&mut self, operations: Vec<ValidationOperation>) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.extend(operations),
        }
    }

    fn add_dpp_operations(&mut self, operations: Vec<ProtocolValidationOperation>) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0
                .operations
                .extend(operations.into_iter().map(ValidationOperation::Protocol)),
        }
    }

    fn operations_consume(self) -> Vec<ValidationOperation> {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations,
        }
    }

    fn operations_slice(&self) -> &[ValidationOperation] {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.as_slice(),
        }
    }

    fn in_dry_run(&self) -> bool {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run,
        }
    }

    fn enable_dry_run(&mut self) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run = true,
        }
    }

    fn disable_dry_run(&mut self) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run = false,
        }
    }
    fn fee_cost(&self, platform_version: &PlatformVersion) -> Result<FeeResult, Error> {
        match self {
            StateTransitionExecutionContext::V0(v0) => {
                let mut fee_result = FeeResult::default();
                ValidationOperation::add_many_to_fee_result(
                    v0.operations.as_slice(),
                    &mut fee_result,
                    platform_version,
                )?;
                Ok(fee_result)
            }
        }
    }
}

impl DefaultForPlatformVersion for StateTransitionExecutionContext {
    type Error = ExecutionError;

    fn default_for_platform_version(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .drive_abci
            .structs
            .state_transition_execution_context
        {
            0 => Ok(StateTransitionExecutionContextV0::default().into()),
            version => Err(ExecutionError::UnknownVersionMismatch {
                method: "DataContract::from_json_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
    use crate::execution::types::execution_operation::RetrieveIdentityInfo;
    use dpp::identity::KeyType;
    use dpp::validation::operations::ProtocolValidationOperation;
    use dpp::version::DefaultForPlatformVersion;

    fn make_context() -> StateTransitionExecutionContext {
        StateTransitionExecutionContext::default_for_platform_version(PlatformVersion::latest())
            .unwrap()
    }

    #[test]
    fn default_context_starts_with_no_operations() {
        let ctx = make_context();
        assert!(ctx.operations_slice().is_empty());
    }

    #[test]
    fn default_context_not_in_dry_run() {
        let ctx = make_context();
        assert!(!ctx.in_dry_run());
    }

    #[test]
    fn enable_disable_dry_run() {
        let mut ctx = make_context();
        ctx.enable_dry_run();
        assert!(ctx.in_dry_run());
        ctx.disable_dry_run();
        assert!(!ctx.in_dry_run());
    }

    #[test]
    fn add_operation_increases_count() {
        let mut ctx = make_context();
        ctx.add_operation(ValidationOperation::SingleSha256(1));
        assert_eq!(ctx.operations_slice().len(), 1);
    }

    #[test]
    fn add_operations_batch() {
        let mut ctx = make_context();
        ctx.add_operations(vec![
            ValidationOperation::SingleSha256(1),
            ValidationOperation::DoubleSha256(2),
        ]);
        assert_eq!(ctx.operations_slice().len(), 2);
    }

    #[test]
    fn add_dpp_operations_wraps_in_protocol() {
        let mut ctx = make_context();
        ctx.add_dpp_operations(vec![
            ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(50),
        ]);
        assert_eq!(ctx.operations_slice().len(), 1);
        assert!(matches!(
            ctx.operations_slice()[0],
            ValidationOperation::Protocol(_)
        ));
    }

    #[test]
    fn operations_consume_returns_all_operations() {
        let mut ctx = make_context();
        ctx.add_operation(ValidationOperation::Ripemd160(1));
        ctx.add_operation(ValidationOperation::SingleSha256(2));
        let ops = ctx.operations_consume();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn fee_cost_empty_context_returns_zero() {
        let ctx = make_context();
        let fee = ctx.fee_cost(PlatformVersion::latest()).unwrap();
        assert_eq!(fee.processing_fee, 0);
        assert_eq!(fee.storage_fee, 0);
    }

    #[test]
    fn fee_cost_with_operations_returns_nonzero() {
        let mut ctx = make_context();
        ctx.add_operation(ValidationOperation::SignatureVerification(
            SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1),
        ));
        ctx.add_operation(ValidationOperation::RetrieveIdentity(
            RetrieveIdentityInfo::only_balance(),
        ));
        let fee = ctx.fee_cost(PlatformVersion::latest()).unwrap();
        assert!(fee.processing_fee > 0);
    }

    #[test]
    fn from_v0_conversion() {
        let v0 = StateTransitionExecutionContextV0::default();
        let ctx: StateTransitionExecutionContext = v0.into();
        assert!(!ctx.in_dry_run());
        assert!(ctx.operations_slice().is_empty());
    }
}
