use crate::state_transition::StateTransitionLike;

use super::{calculate_operations_fees, constants::DEFAULT_USER_TIP};

pub fn calculate_state_transition_fee(state_transition: &impl StateTransitionLike) -> i64 {
    let execution_context = state_transition.get_execution_context();
    let fee = calculate_operations_fees(execution_context.get_operations());

    // Is not implemented yet
    let storage_refund = 0;

    (fee.storage + fee.processing) + DEFAULT_USER_TIP - storage_refund
}

#[cfg(test)]
mod test {
    use crate::{
        identity::{
            state_transition::identity_create_transition::IdentityCreateTransition, KeyType,
        },
        state_transition::{
            fee::operations::{
                DeleteOperation, Operation, PreCalculatedOperation, ReadOperation, WriteOperation,
            },
            state_transition_execution_context::StateTransitionExecutionContext,
            StateTransitionLike,
        },
        tests::fixtures::identity_create_transition_fixture_json,
        NativeBlsModule,
    };

    use super::calculate_state_transition_fee;

    // TODO: Must be more comprehensive. After we settle all factors and formula.
    #[test]
    fn should_calculate_fee_based_on_executed_operations() {
        let bls = NativeBlsModule::default();
        let private_key =
            hex::decode("af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837")
                .unwrap();
        let mut state_transition =
            IdentityCreateTransition::new(identity_create_transition_fixture_json(None)).unwrap();
        state_transition
            .sign_by_private_key(&private_key, KeyType::ECDSA_SECP256K1, &bls)
            .expect("signing should be successful");

        let execution_context = StateTransitionExecutionContext::default();
        execution_context.add_operation(Operation::Read(ReadOperation::new(10)));
        execution_context.add_operation(Operation::Write(WriteOperation::new(5, 5)));
        execution_context.add_operation(Operation::Delete(DeleteOperation::new(6, 6)));
        execution_context.add_operation(Operation::PreCalculated(PreCalculatedOperation::new(
            12, 12,
        )));
        state_transition.set_execution_context(execution_context);

        let result = calculate_state_transition_fee(&state_transition);
        assert_eq!(13616, result)
    }
}
