use crate::consensus::basic::identity::IdentityAssetLockTransactionTooManyInputsError;
use crate::validation::SimpleConsensusValidationResult;
use dashcore::Transaction;
use platform_version::version::PlatformVersion;

/// Refuses an asset lock transaction with more inputs than `max_asset_lock_transaction_inputs`.
///
/// An instant asset lock proof embeds its transaction, and every input of it, in the state
/// transition, so the cap keeps that transition under `max_state_transition_size`. Consensus
/// runs this check on an instant proof (`validate_instant_asset_lock_proof_structure` 1), at
/// every protocol version selecting that generation: editing this body changes their
/// verdicts, so a different check needs a new generation of that method instead. Dash Core's
/// relay policy refuses the same bound of 100 (`MAX_STANDARD_ASSET_LOCK_INPUTS`), which is why
/// wallets run this check before broadcasting an asset lock.
pub fn validate_asset_lock_transaction_inputs(
    transaction: &Transaction,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    let max_inputs = platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .max_asset_lock_transaction_inputs;
    let input_count = transaction.input.len();
    if input_count > max_inputs as usize {
        SimpleConsensusValidationResult::new_with_error(
            IdentityAssetLockTransactionTooManyInputsError::new(
                u16::try_from(input_count).unwrap_or(u16::MAX),
                max_inputs,
            )
            .into(),
        )
    } else {
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::tests::fixtures::instant_asset_lock_proof_transaction_fixture_with_inputs;
    use dashcore::TxIn;

    fn too_many_inputs(result: &SimpleConsensusValidationResult) -> (u16, u16) {
        match result.errors.as_slice() {
            [ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionTooManyInputsError(error),
            )] => (error.actual_inputs(), error.max_inputs()),
            errors => panic!("expected only the too many inputs error, got {errors:?}"),
        }
    }

    #[test]
    fn should_accept_a_transaction_at_the_input_cap() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 100);

        let result =
            validate_asset_lock_transaction_inputs(&transaction, PlatformVersion::latest());

        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_transaction_over_the_input_cap() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);

        let result =
            validate_asset_lock_transaction_inputs(&transaction, PlatformVersion::latest());

        assert_eq!(too_many_inputs(&result), (101, 100));
    }

    /// The bound is the protocol version's table value: before protocol 11 it is `u16::MAX`,
    /// so a transaction over today's cap of 100 passes there.
    #[test]
    fn should_accept_a_transaction_over_100_inputs_where_the_table_does_not_cap_them() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);

        let result = validate_asset_lock_transaction_inputs(
            &transaction,
            PlatformVersion::get(10).expect("protocol 10 exists"),
        );

        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    /// The error carries the input count as a `u16`, so a count past it reports `u16::MAX`.
    #[test]
    fn should_report_an_input_count_past_u16_max_as_u16_max() {
        let mut transaction =
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 1);
        transaction.input = vec![TxIn::default(); usize::from(u16::MAX) + 1];

        let result =
            validate_asset_lock_transaction_inputs(&transaction, PlatformVersion::latest());

        assert_eq!(too_many_inputs(&result), (u16::MAX, 100));
    }
}
