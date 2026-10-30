use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionOutputNotFoundError, InvalidIdentityAssetLockTransactionError,
    InvalidIdentityAssetLockTransactionOutputError,
};
use crate::validation::ConsensusValidationResult;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{Transaction, TxOut};

/// Validates asset lock transaction structure: an asset lock payload whose credit output at
/// `output_index` exists and is P2PKH.
///
/// Unlike v0 it does not count inputs. `max_asset_lock_transaction_inputs` bounds the size of a
/// state transition that embeds the whole transaction, so it is checked by the instant asset
/// lock proof, the one proof that embeds it. A chain asset lock proof names only the outpoint.
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
    use dashcore::hashes::Hash;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::{OutPoint, PubkeyHash, ScriptBuf, TxIn, Txid};

    fn asset_lock_transaction(input_count: usize, credit_output_script: ScriptBuf) -> Transaction {
        Transaction {
            version: 3,
            lock_time: 0,
            input: (0..input_count)
                .map(|i| TxIn {
                    previous_output: OutPoint::new(Txid::from_byte_array([1; 32]), i as u32),
                    ..Default::default()
                })
                .collect(),
            output: vec![TxOut {
                value: 100_000_000,
                script_pubkey: ScriptBuf::new_op_return(&[]),
            }],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![TxOut {
                        value: 100_000_000,
                        script_pubkey: credit_output_script,
                    }],
                },
            )),
        }
    }

    fn p2pkh() -> ScriptBuf {
        ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20]))
    }

    #[test]
    fn should_accept_a_transaction_with_more_inputs_than_the_instant_proof_cap() {
        let result =
            validate_asset_lock_transaction_structure_v1(&asset_lock_transaction(121, p2pkh()), 0);

        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    #[test]
    fn should_reject_a_transaction_without_an_asset_lock_payload() {
        let mut transaction = asset_lock_transaction(121, p2pkh());
        transaction.special_transaction_payload = None;

        let result = validate_asset_lock_transaction_structure_v1(&transaction, 0);

        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityAssetLockTransactionError(_)
            )]
        ));
    }

    #[test]
    fn should_reject_a_missing_credit_output() {
        let result =
            validate_asset_lock_transaction_structure_v1(&asset_lock_transaction(121, p2pkh()), 1);

        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutputNotFoundError(_)
            )]
        ));
    }

    #[test]
    fn should_reject_a_credit_output_that_is_not_p2pkh() {
        let result = validate_asset_lock_transaction_structure_v1(
            &asset_lock_transaction(121, ScriptBuf::new_op_return(&[])),
            0,
        );

        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityAssetLockTransactionOutputError(_)
            )]
        ));
    }
}
