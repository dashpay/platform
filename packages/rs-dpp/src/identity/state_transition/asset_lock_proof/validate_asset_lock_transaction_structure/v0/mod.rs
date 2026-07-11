use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionOutputNotFoundError,
    IdentityAssetLockTransactionTooManyInputsError, InvalidIdentityAssetLockTransactionError,
    InvalidIdentityAssetLockTransactionOutputError,
};
use crate::validation::ConsensusValidationResult;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{Transaction, TxOut};

/// Validates asset lock transaction structure
#[inline(always)]
pub(super) fn validate_asset_lock_transaction_structure_v0(
    transaction: &Transaction,
    output_index: u32,
    max_inputs: u16,
) -> ConsensusValidationResult<TxOut> {
    let input_count = transaction.input.len();
    if input_count > max_inputs as usize {
        return ConsensusValidationResult::new_with_error(
            IdentityAssetLockTransactionTooManyInputsError::new(
                u16::try_from(input_count).unwrap_or(u16::MAX),
                max_inputs,
            )
            .into(),
        );
    }

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
        //Todo: better error
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
    use dashcore::secp256k1::rand::thread_rng;
    use dashcore::secp256k1::Secp256k1;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::{Network, OutPoint, PrivateKey, ScriptBuf, TxIn, Txid};
    use std::str::FromStr;

    fn make_asset_lock_transaction(num_inputs: usize) -> Transaction {
        let secp = Secp256k1::new();
        let mut rng = thread_rng();

        let input_secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
        let private_key = PrivateKey::new(input_secret_key, Network::Testnet);
        let public_key = private_key.public_key(&secp);
        let public_key_hash = public_key.pubkey_hash();

        let secret_key = dashcore::secp256k1::SecretKey::new(&mut rng);
        let one_time_private_key = PrivateKey::new(secret_key, Network::Testnet);
        let one_time_public_key = one_time_private_key.public_key(&secp);
        let one_time_key_hash = one_time_public_key.pubkey_hash();

        let base_txid =
            Txid::from_str("a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458")
                .unwrap();

        let inputs: Vec<TxIn> = (0..num_inputs)
            .map(|i| TxIn {
                previous_output: OutPoint::new(base_txid, i as u32),
                script_sig: ScriptBuf::new_p2pkh(&public_key_hash),
                sequence: 0,
                witness: Default::default(),
            })
            .collect();

        let funding_output = TxOut {
            value: 100_000_000,
            script_pubkey: ScriptBuf::new_p2pkh(&one_time_key_hash),
        };

        let burn_output = TxOut {
            value: 100_000_000,
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };

        let payload = TransactionPayload::AssetLockPayloadType(AssetLockPayload {
            version: 0,
            credit_outputs: vec![funding_output],
        });

        Transaction {
            version: 3,
            lock_time: 0,
            input: inputs,
            output: vec![burn_output],
            special_transaction_payload: Some(payload),
        }
    }

    #[test]
    fn should_reject_transaction_with_too_many_inputs() {
        let tx = make_asset_lock_transaction(101);
        let result = validate_asset_lock_transaction_structure_v0(&tx, 0, 100);

        assert!(result.errors.len() == 1);
        let error = &result.errors[0];
        match error {
            crate::consensus::ConsensusError::BasicError(
                crate::consensus::basic::BasicError::IdentityAssetLockTransactionTooManyInputsError(
                    e,
                ),
            ) => {
                assert_eq!(e.actual_inputs(), 101);
                assert_eq!(e.max_inputs(), 100);
            }
            other => {
                panic!("Expected IdentityAssetLockTransactionTooManyInputsError, got: {other:?}")
            }
        }
    }

    #[test]
    fn should_accept_transaction_at_max_inputs() {
        let tx = make_asset_lock_transaction(100);
        let result = validate_asset_lock_transaction_structure_v0(&tx, 0, 100);

        let has_too_many_inputs_error = result.errors.iter().any(|e| {
            matches!(
                e,
                crate::consensus::ConsensusError::BasicError(
                    crate::consensus::basic::BasicError::IdentityAssetLockTransactionTooManyInputsError(_)
                )
            )
        });
        assert!(
            !has_too_many_inputs_error,
            "Should not reject exactly 100 inputs"
        );
        assert!(
            result.is_valid(),
            "Transaction at max inputs should be valid"
        );
    }

    #[test]
    fn should_accept_transaction_with_one_input() {
        let tx = make_asset_lock_transaction(1);
        let result = validate_asset_lock_transaction_structure_v0(&tx, 0, 100);

        let has_too_many_inputs_error = result.errors.iter().any(|e| {
            matches!(
                e,
                crate::consensus::ConsensusError::BasicError(
                    crate::consensus::basic::BasicError::IdentityAssetLockTransactionTooManyInputsError(_)
                )
            )
        });
        assert!(!has_too_many_inputs_error, "Should not reject single input");
        assert!(
            result.is_valid(),
            "Single input transaction should be valid"
        );
    }
}
