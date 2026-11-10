use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionIsNotFoundError, IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofTransactionHeightError,
};
use dpp::dashcore::TxOut;
use dpp::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;
use dpp::prelude::{AssetLockProof, ConsensusValidationResult};
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

/// This fetches the asset lock transaction output from core
pub fn fetch_asset_lock_transaction_output_sync_v0<C: CoreRPCLike>(
    core_rpc: &C,
    asset_lock_proof: &AssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => {
            if let Some(output) = proof.output() {
                Ok(ValidationResult::new_with_data(output.clone()))
            } else {
                Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        proof.output_index() as usize
                    )
                    .into(),
                ))
            }
        }
        AssetLockProof::Chain(proof) => {
            let output_index = proof.out_point.vout;
            let transaction_hash = proof.out_point.txid;

            // Fetch transaction

            let maybe_transaction_info = core_rpc
                .get_optional_transaction_extended_info(&transaction_hash)
                .map_err(|e| {
                    //todo multiple errors are possible from core, some that
                    // would lead to consensus errors, other execution errors
                    Error::Execution(ExecutionError::DashCoreBadResponseError(format!(
                        "can't fetch asset transaction for chain asset lock proof: {e}",
                    )))
                })?;

            let Some(transaction_info) = maybe_transaction_info else {
                // Transaction hash bytes needs to be reversed to match actual transaction hash
                let mut hash: [u8; 32] = transaction_hash.as_raw_hash().to_byte_array();
                hash.reverse();

                return Ok(ValidationResult::new_with_error(
                    IdentityAssetLockTransactionIsNotFoundError::new(hash).into(),
                ));
            };

            // Make sure transaction is mined on the chain locked block or before

            let Some(transaction_height) = transaction_info.height else {
                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidAssetLockProofTransactionHeightError::new(
                        proof.core_chain_locked_height,
                        None,
                    )
                    .into(),
                ));
            };

            let is_transaction_not_mined = transaction_height == -1;
            let transaction_height = transaction_height as u32;

            // Return an error if transaction is not mined
            // or if it is mined after the chain locked height
            if is_transaction_not_mined || transaction_height > proof.core_chain_locked_height {
                let reported_height = if is_transaction_not_mined {
                    None
                } else {
                    Some(transaction_height)
                };

                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidAssetLockProofTransactionHeightError::new(
                        proof.core_chain_locked_height,
                        reported_height,
                    )
                    .into(),
                ));
            }

            let transaction = transaction_info
                .transaction()
                .map_err(|e| Error::Execution(ExecutionError::DashCoreConsensusEncodeError(e)))?;

            // Validate asset lock transaction

            // While we don't need this validation for recheck, we still need to get the tx_out
            // To get the Tx_out we need some sanity checks, then this checks that we are p2pkh.
            // The check for p2pkh is only marginally more expensive than the check to see if we are
            // on a recheck, so there's no point making the code more complicated and stripping
            // out a very cheap check on recheck tx
            validate_asset_lock_transaction_structure(&transaction, output_index, platform_version)
                .map_err(Error::Protocol)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::consensus::serialize;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dpp::dashcore::transaction::special_transaction::TransactionPayload;
    use dpp::dashcore::{BlockHash, ScriptBuf, Transaction, TxIn, Txid};
    use dpp::dashcore_rpc::json::GetRawTransactionResult;
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dpp::tests::fixtures::instant_asset_lock_proof_transaction_fixture_with_inputs;

    fn dummy_txid(seed: u8) -> Txid {
        Txid::from_raw_hash(dpp::dashcore::hashes::sha256d::Hash::from_byte_array(
            [seed; 32],
        ))
    }

    /// A chain proof for output 0 of `transaction`, chain locked at height 42, and a Core that
    /// reports `transaction` mined at height 40.
    fn chain_proof_of_mined_transaction(
        transaction: &Transaction,
    ) -> (AssetLockProof, MockCoreRPCLike) {
        let mut out_point_bytes = [0u8; 36];
        out_point_bytes[..32].copy_from_slice(transaction.txid().as_raw_hash().as_byte_array());
        out_point_bytes[32..36].copy_from_slice(&0u32.to_le_bytes());
        let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof::new(42, out_point_bytes));

        let transaction = transaction.clone();
        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_optional_transaction_extended_info()
            .returning(move |_txid| {
                Ok(Some(GetRawTransactionResult {
                    in_active_chain: true,
                    hex: serialize(&transaction),
                    txid: transaction.txid(),
                    size: 0,
                    version: transaction.version as u32,
                    tx_type: 8,
                    locktime: transaction.lock_time,
                    vin: vec![],
                    vout: vec![],
                    extra_payload_size: None,
                    extra_payload: None,
                    blockhash: Some(BlockHash::all_zeros()),
                    confirmations: Some(3),
                    time: Some(0),
                    blocktime: Some(0),
                    height: Some(40),
                    instantlock: false,
                    instantlock_internal: false,
                    chainlock: true,
                }))
            });

        (asset_lock_proof, core_rpc)
    }

    #[test]
    fn instant_proof_with_missing_output_returns_output_not_found() {
        let platform_version = PlatformVersion::latest();

        // InstantAssetLockProof with a vout that doesn't exist in the transaction's tx outs.
        let tx_without_output = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![TxIn::default()],
            output: vec![], // no outputs at all
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![],
                },
            )),
        };

        let proof = InstantAssetLockProof::new(
            dpp::dashcore::InstantLock {
                version: 1,
                inputs: vec![],
                txid: tx_without_output.txid(),
                cyclehash: [0u8; 32].into(),
                signature: [0u8; 96].into(),
            },
            tx_without_output,
            5, // output_index out of range
        );

        let asset_lock_proof = AssetLockProof::Instant(proof);

        let core_rpc = MockCoreRPCLike::new();
        let result = fetch_asset_lock_transaction_output_sync_v0(
            &core_rpc,
            &asset_lock_proof,
            platform_version,
        )
        .expect("should not return Err");

        assert!(!result.is_valid(), "should be invalid");
        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutputNotFoundError(_)
                )
            )),
            "expected IdentityAssetLockTransactionOutputNotFoundError, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn chain_proof_tx_not_found_returns_tx_not_found_error() {
        let platform_version = PlatformVersion::latest();

        let txid = dummy_txid(0x11);
        let mut out_point_bytes = [0u8; 36];
        out_point_bytes[..32].copy_from_slice(txid.as_raw_hash().as_byte_array());
        out_point_bytes[32..36].copy_from_slice(&0u32.to_le_bytes());

        let chain_proof = ChainAssetLockProof::new(42, out_point_bytes);
        let asset_lock_proof = AssetLockProof::Chain(chain_proof);

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_optional_transaction_extended_info()
            .returning(|_txid| Ok(None));

        let result = fetch_asset_lock_transaction_output_sync_v0(
            &core_rpc,
            &asset_lock_proof,
            platform_version,
        )
        .expect("should not return Err");

        assert!(!result.is_valid(), "should be invalid");
        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionIsNotFoundError(_)
                )
            )),
            "expected IdentityAssetLockTransactionIsNotFoundError, got: {:?}",
            result.errors
        );
    }

    /// A chain proof carries only an outpoint and a height, so the input cap that keeps an
    /// instant proof's state transition small does not hold its transaction from protocol 15:
    /// a mined asset lock of more than 100 inputs, which no instant proof can redeem, is
    /// redeemable with a chain proof.
    #[test]
    fn should_return_the_credit_output_of_a_chain_proof_over_the_input_cap() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);
        let (asset_lock_proof, core_rpc) = chain_proof_of_mined_transaction(&transaction);

        let result = fetch_asset_lock_transaction_output_sync_v0(
            &core_rpc,
            &asset_lock_proof,
            PlatformVersion::latest(),
        )
        .expect("should not return Err");

        assert!(result.is_valid(), "got {:?}", result.errors);
        let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &transaction.special_transaction_payload
        else {
            panic!("the fixture is an asset lock");
        };
        assert_eq!(result.data.as_ref(), payload.credit_outputs.first());
    }

    /// Dropping the cap leaves the rest of the transaction's structure checked: a chain proof
    /// over the cap whose credit output is not P2PKH is still refused for that.
    #[test]
    fn should_refuse_a_chain_proof_over_the_input_cap_whose_credit_output_is_not_p2pkh() {
        let mut transaction =
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);
        let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &mut transaction.special_transaction_payload
        else {
            panic!("the fixture is an asset lock");
        };
        payload.credit_outputs[0].script_pubkey = ScriptBuf::new_op_return(&[]);
        let (asset_lock_proof, core_rpc) = chain_proof_of_mined_transaction(&transaction);

        let result = fetch_asset_lock_transaction_output_sync_v0(
            &core_rpc,
            &asset_lock_proof,
            PlatformVersion::latest(),
        )
        .expect("should not return Err");

        assert!(
            matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityAssetLockTransactionOutputError(_)
                )]
            ),
            "got {:?}",
            result.errors
        );
    }

    /// Protocol 14 and earlier keep refusing such a chain proof, so the blocks committed under
    /// them replay with the same verdicts.
    #[test]
    fn should_refuse_a_chain_proof_over_the_input_cap_at_protocol_14() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);
        let (asset_lock_proof, core_rpc) = chain_proof_of_mined_transaction(&transaction);

        let result = fetch_asset_lock_transaction_output_sync_v0(
            &core_rpc,
            &asset_lock_proof,
            PlatformVersion::get(14).expect("protocol 14 exists"),
        )
        .expect("should not return Err");

        match result.errors.as_slice() {
            [ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionTooManyInputsError(error),
            )] => {
                assert_eq!(error.actual_inputs(), 101);
                assert_eq!(error.max_inputs(), 100);
            }
            errors => panic!("expected only the too many inputs error, got {errors:?}"),
        }
    }
}
