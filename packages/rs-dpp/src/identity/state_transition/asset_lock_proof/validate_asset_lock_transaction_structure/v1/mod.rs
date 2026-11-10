use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionOutputNotFoundError, InvalidIdentityAssetLockTransactionError,
    InvalidIdentityAssetLockTransactionOutputError,
};
use crate::validation::ConsensusValidationResult;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{Transaction, TxOut};

/// Validates asset lock transaction structure.
///
/// Unlike v0, it does not cap the transaction's inputs. Drive runs it on a chain proof's
/// transaction, which it fetches from Core, and a chain proof carries only an outpoint and a
/// height whatever the transaction's size. The cap applies to the transaction an instant proof
/// embeds, and `validate_instant_asset_lock_proof_structure` 1 checks it there.
#[inline(always)]
pub(super) fn validate_asset_lock_transaction_structure_v1(
    transaction: &Transaction,
    output_index: u32,
) -> ConsensusValidationResult<TxOut> {
    // It must be an Asset Lock Special Transaction
    let Some(TransactionPayload::AssetLockPayloadType(ref payload)) =
        transaction.special_transaction_payload
    else {
        return ConsensusValidationResult::new_with_error(
            InvalidIdentityAssetLockTransactionError::new(
                "Funding transaction must have an Asset Lock Special Transaction Payload",
            )
            .into(),
        );
    };

    // Output index should point to existing funding output in payload
    let Some(output) = payload.credit_outputs.get(output_index as usize) else {
        return ConsensusValidationResult::new_with_error(
            IdentityAssetLockTransactionOutputNotFoundError::new(output_index as usize).into(),
        );
    };

    // Output should be P2PKH
    if !output.script_pubkey.is_p2pkh() {
        ConsensusValidationResult::new_with_error(
            InvalidIdentityAssetLockTransactionOutputError::new(output_index as usize).into(),
        )
    } else {
        ConsensusValidationResult::new_with_data(output.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::tests::fixtures::instant_asset_lock_proof_transaction_fixture_with_inputs;

    #[test]
    fn should_accept_a_transaction_over_the_input_cap() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);

        let result = validate_asset_lock_transaction_structure_v1(&transaction, 0);

        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_transaction_without_an_asset_lock_payload() {
        let mut transaction =
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);
        transaction.special_transaction_payload = None;

        let result = validate_asset_lock_transaction_structure_v1(&transaction, 0);

        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityAssetLockTransactionError(_)
            )]
        ));
    }
}
