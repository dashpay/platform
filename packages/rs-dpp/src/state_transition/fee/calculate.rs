use crate::credits::Credits;
use crate::prelude::Identifier;
use crate::state_transition::fee::constants::{BASE_ST_FEE, DEFAULT_USER_TIP};
use crate::state_transition::fee::operations::aggregate_operation_fees::aggregate_operation_fees;
use crate::state_transition::fee::ExecutionFees;
use crate::state_transition::{StateTransition, StateTransitionLike};
use crate::ProtocolError;

struct StateTransitionFees {
    identity_id: Identifier,
    desired_fee: Credits,
    required_fee: Credits,
    fee_result: ExecutionFees,
}

impl StateTransitionFees {
    pub fn calculate(state_transition: &StateTransition) -> Result<Self, ProtocolError> {
        let execution_context = state_transition.get_execution_context();
        let operations = execution_context.get_operations();

        let fee_result = aggregate_operation_fees(&operations)?;

        let base_fee = BASE_ST_FEE
            .checked_add(DEFAULT_USER_TIP)
            .ok_or(ProtocolError::Overflow("base fees overflow"))?;

        let required_fee = base_fee
            .checked_add(fee_result.storage_fee)
            .ok_or(ProtocolError::Overflow("required fee overflow"))?;

        let desired_fee = base_fee
            .checked_add(fee_result.storage_fee)
            .ok_or(ProtocolError::Overflow("desired fee overflow"))?
            .checked_add(fee_result.processing_fee)
            .ok_or(ProtocolError::Overflow("desired fee overflow"))?;

        Ok(Self {
            identity_id: *identity_id,
            required_fee,
            desired_fee,
            fee_result,
        })
    }

    pub fn deduct_balance_debt(&mut self, debt: Credits) {
        self.desired_fee - debt
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn desired_fee(&self) -> Credits {
        self.desired_fee
    }

    pub fn required_fee(&self) -> Credits {
        self.required_fee
    }

    pub fn total_fee(&self) -> Credits {
        self.desired_fee
    }

    pub fn total_refund(&self) -> Credits {
        self.fee_result.fee_refunds.sum()
    }

    pub fn fee_result(&self) -> &ExecutionFees {
        &self.fee_result
    }
}
