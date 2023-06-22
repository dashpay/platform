use dpp::consensus::basic::identity::{
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
};

use dpp::identity::state_transition::identity_credit_withdrawal_transition::{IdentityCreditWithdrawalTransition, Pooling};
use dpp::identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR;
use dpp::util::is_fibonacci_number::is_fibonacci_number;
use dpp::validation::SimpleConsensusValidationResult;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = validate_schema_v0(
            &IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR,
            self,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        //todo: version validation
        //Ok(validate_protocol_version(self.protocol_version))

        // currently we do not support pooling, so we must validate that pooling is `Never`

        if self.pooling != Pooling::Never {
            result.add_error(
                NotImplementedIdentityCreditWithdrawalTransitionPoolingError::new(
                    self.pooling as u8,
                ),
            );

            return Ok(result);
        }

        // validate core_fee is in fibonacci sequence

        if !is_fibonacci_number(self.core_fee_per_byte) {
            result.add_error(InvalidIdentityCreditWithdrawalTransitionCoreFeeError::new(
                self.core_fee_per_byte,
            ));

            return Ok(result);
        }

        // validate output_script types
        if !self.output_script.is_p2pkh() && !self.output_script.is_p2sh() {
            result.add_error(
                InvalidIdentityCreditWithdrawalTransitionOutputScriptError::new(
                    self.output_script.clone(),
                ),
            );
        }

        Ok(result)
    }
}
