use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::TxOut;
use dpp::prelude::{AssetLockProof, ConsensusValidationResult};
use dpp::version::PlatformVersion;

mod v0;

/// This fetches the asset lock transaction output from core
pub fn fetch_asset_lock_transaction_output_sync<C: CoreRPCLike>(
    core_rpc: &C,
    asset_lock_proof: &AssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .asset_locks
        .fetch_asset_lock_transaction_output_sync
    {
        0 => v0::fetch_asset_lock_transaction_output_sync_v0(
            core_rpc,
            asset_lock_proof,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "fetch_asset_lock_transaction_output_sync".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::consensus::serialize;
    use dpp::dashcore::hashes::{sha256d, Hash};
    use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dpp::dashcore::transaction::special_transaction::TransactionPayload;
    use dpp::dashcore::{BlockHash, OutPoint, PubkeyHash, ScriptBuf, Transaction, TxIn, Txid};
    use dpp::dashcore_rpc::json::GetRawTransactionResult;
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::version::v15::PROTOCOL_VERSION_15;
    use platform_version::version::LATEST_VERSION;

    const MINED_AT: u32 = 1_000;

    fn asset_lock_transaction(input_count: usize) -> Transaction {
        let input = (0..input_count)
            .map(|i| TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_raw_hash(sha256d::Hash::hash(&(i as u64).to_le_bytes())),
                    vout: 0,
                },
                ..Default::default()
            })
            .collect();
        Transaction {
            version: 3,
            lock_time: 0,
            input,
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![TxOut {
                        value: 100_000_000,
                        script_pubkey: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20])),
                    }],
                },
            )),
        }
    }

    fn core_with_mined(transaction: Transaction) -> MockCoreRPCLike {
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
                    blockhash: Some(BlockHash::from_byte_array([1; 32])),
                    confirmations: Some(10),
                    time: Some(0),
                    blocktime: Some(0),
                    height: Some(MINED_AT as i32),
                    instantlock: false,
                    instantlock_internal: false,
                    chainlock: true,
                }))
            });
        core_rpc
    }

    fn chain_proof_for(transaction: &Transaction) -> AssetLockProof {
        let mut out_point = [0u8; 36];
        out_point[..32].copy_from_slice(transaction.txid().as_raw_hash().as_byte_array());
        AssetLockProof::Chain(ChainAssetLockProof::new(MINED_AT, out_point))
    }

    fn fetch_chain_proof_output(
        input_count: usize,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<TxOut> {
        let transaction = asset_lock_transaction(input_count);
        let proof = chain_proof_for(&transaction);
        fetch_asset_lock_transaction_output_sync(
            &core_with_mined(transaction),
            &proof,
            platform_version,
        )
        .expect("should not return Err")
    }

    fn is_too_many_inputs(result: &ConsensusValidationResult<TxOut>) -> bool {
        result.errors.iter().any(|e| {
            matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionTooManyInputsError(_)
                )
            )
        })
    }

    /// Protocol versions 11 to 14 hold the transaction behind a chain asset lock proof to
    /// `max_asset_lock_transaction_inputs` (100); earlier versions have no cap, and from
    /// version 15 the cap applies to instant asset lock proofs only.
    #[test]
    fn should_hold_a_chain_proof_to_the_input_cap_only_at_protocol_versions_11_to_14() {
        for protocol_version in 1..=LATEST_VERSION {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("known protocol version");
            let capped = (11..PROTOCOL_VERSION_15).contains(&protocol_version);

            let at_cap = fetch_chain_proof_output(100, platform_version);
            assert!(
                at_cap.is_valid(),
                "protocol {protocol_version}: 100 inputs must be accepted, got {:?}",
                at_cap.errors
            );

            let over_cap = fetch_chain_proof_output(121, platform_version);
            if capped {
                assert!(
                    is_too_many_inputs(&over_cap),
                    "protocol {protocol_version}: 121 inputs must be refused, got {:?}",
                    over_cap.errors
                );
            } else {
                assert!(
                    over_cap.is_valid(),
                    "protocol {protocol_version}: 121 inputs must be accepted, got {:?}",
                    over_cap.errors
                );
            }
        }
    }

    #[test]
    fn should_keep_the_other_structure_checks_for_a_chain_proof_over_the_input_cap() {
        let mut transaction = asset_lock_transaction(121);
        transaction.special_transaction_payload = None;
        let proof = chain_proof_for(&transaction);

        let result = fetch_asset_lock_transaction_output_sync(
            &core_with_mined(transaction),
            &proof,
            PlatformVersion::latest(),
        )
        .expect("should not return Err");

        assert!(
            result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(BasicError::InvalidIdentityAssetLockTransactionError(_))
            )),
            "expected InvalidIdentityAssetLockTransactionError, got: {:?}",
            result.errors
        );
    }
}
