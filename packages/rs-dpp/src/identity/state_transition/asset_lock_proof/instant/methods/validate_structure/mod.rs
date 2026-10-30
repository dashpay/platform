mod v0;
mod v1;

#[allow(unused_imports)] // Removing causes build failures; yet clippy insists it's unused
pub(in crate::identity::state_transition::asset_lock_proof::instant) use v0::validate_instant_asset_lock_proof_structure_v0;
#[allow(unused_imports)]
pub(in crate::identity::state_transition::asset_lock_proof::instant) use v1::validate_instant_asset_lock_proof_structure_v1;

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dashcore::hashes::Hash;
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{InstantLock, OutPoint, PubkeyHash, ScriptBuf, Transaction, TxIn, TxOut, Txid};
    use platform_version::version::{PlatformVersion, LATEST_VERSION};

    fn instant_proof(input_count: usize) -> InstantAssetLockProof {
        let transaction = Transaction {
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
                        script_pubkey: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20])),
                    }],
                },
            )),
        };
        InstantAssetLockProof::new(
            InstantLock {
                version: 1,
                inputs: vec![],
                txid: transaction.txid(),
                cyclehash: [0u8; 32].into(),
                signature: [0u8; 96].into(),
            },
            transaction,
            0,
        )
    }

    /// The instant lock must name the transaction the proof embeds; that is checked before the
    /// input count, so a proof with both faults reports the mismatch.
    #[test]
    fn should_report_a_locked_transaction_mismatch_before_the_input_count() {
        let mut proof = instant_proof(121);
        proof.instant_lock.txid = Txid::from_byte_array([9; 32]);

        let result = proof
            .validate_structure(PlatformVersion::latest())
            .expect("should not return Err");

        assert!(
            matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                )]
            ),
            "got {:?}",
            result.errors
        );
    }

    /// Protocol versions 1 to 10 have no input cap; from version 11 an instant asset lock proof
    /// holds the transaction it embeds to `max_asset_lock_transaction_inputs` (100), whichever
    /// check carries it.
    #[test]
    fn should_hold_an_instant_proof_to_the_input_cap_from_protocol_version_11() {
        for protocol_version in 1..=LATEST_VERSION {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("known protocol version");

            let at_cap = instant_proof(100)
                .validate_structure(platform_version)
                .expect("should not return Err");
            assert!(
                at_cap.is_valid(),
                "protocol {protocol_version}: 100 inputs must be accepted, got {:?}",
                at_cap.errors
            );

            let over_cap = instant_proof(121)
                .validate_structure(platform_version)
                .expect("should not return Err");
            if protocol_version >= 11 {
                assert!(
                    matches!(
                        over_cap.errors.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::IdentityAssetLockTransactionTooManyInputsError(e)
                        )] if e.actual_inputs() == 121 && e.max_inputs() == 100
                    ),
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
}
