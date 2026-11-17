mod v0;
mod v1;

#[allow(unused_imports)] // Removing causes build failures; yet clippy insists it's unused
pub(in crate::identity::state_transition::asset_lock_proof::instant) use v0::validate_instant_asset_lock_proof_structure_v0;
#[allow(unused_imports)] // Removing causes build failures; yet clippy insists it's unused
pub(in crate::identity::state_transition::asset_lock_proof::instant) use v1::validate_instant_asset_lock_proof_structure_v1;

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use crate::tests::fixtures::{
        instant_asset_lock_is_lock_fixture,
        instant_asset_lock_proof_transaction_fixture_with_inputs,
    };
    use dashcore::transaction::special_transaction::TransactionPayload;
    use dashcore::{ScriptBuf, Transaction, Txid};
    use platform_version::version::PlatformVersion;
    use std::str::FromStr;

    fn proof(transaction: Transaction, output_index: u32) -> InstantAssetLockProof {
        let instant_lock = instant_asset_lock_is_lock_fixture(transaction.txid());
        InstantAssetLockProof::new(instant_lock, transaction, output_index)
    }

    /// A malformed proof, named, with the error it is expected to be refused with.
    type MalformedProofCase = (&'static str, InstantAssetLockProof, fn(&BasicError) -> bool);

    fn protocol_14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("protocol 14 exists")
    }

    /// An instant proof embeds its transaction in the state transition, so the input cap that
    /// keeps the transition under its size limit holds for it on both sides of protocol 15,
    /// which moved the cap out of the transaction structure check.
    #[test]
    fn should_refuse_an_instant_proof_over_the_input_cap_before_and_from_protocol_15() {
        let over_cap = proof(
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101),
            0,
        );
        let at_cap = proof(
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 100),
            0,
        );

        for platform_version in [protocol_14(), PlatformVersion::latest()] {
            let result = over_cap
                .validate_structure(platform_version)
                .expect("a known validator");
            match result.errors.as_slice() {
                [ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionTooManyInputsError(error),
                )] => {
                    assert_eq!(error.actual_inputs(), 101);
                    assert_eq!(error.max_inputs(), 100);
                }
                errors => panic!(
                    "protocol {}: expected only the too many inputs error, got {errors:?}",
                    platform_version.protocol_version
                ),
            }

            let result = at_cap
                .validate_structure(platform_version)
                .expect("a known validator");
            assert!(
                result.is_valid(),
                "protocol {}: got {:?}",
                platform_version.protocol_version,
                result.errors
            );
        }
    }

    /// Protocol 15 reorders nothing an instant proof sees: a proof that breaks several rules
    /// at once is refused with the same single error before and after the cap moved.
    #[test]
    fn should_refuse_a_malformed_instant_proof_with_the_same_error_before_and_from_protocol_15() {
        let over_cap = || instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);

        let mut txid_mismatch = proof(over_cap(), 0);
        txid_mismatch.instant_lock.txid =
            Txid::from_str("6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d")
                .expect("valid txid");

        let mut no_payload = over_cap();
        no_payload.special_transaction_payload = None;

        let mut not_p2pkh =
            instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 100);
        let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &mut not_p2pkh.special_transaction_payload
        else {
            panic!("the fixture is an asset lock");
        };
        payload.credit_outputs[0].script_pubkey = ScriptBuf::new_op_return(&[]);

        let at_cap = || instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 100);

        // In the order the checks run: txid, input cap, payload, credit output, P2PKH.
        let cases: [MalformedProofCase; 5] = [
            ("txid mismatch over the cap", txid_mismatch, |error| {
                matches!(
                    error,
                    BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                )
            }),
            ("no payload over the cap", proof(no_payload, 0), |error| {
                matches!(
                    error,
                    BasicError::IdentityAssetLockTransactionTooManyInputsError(_)
                )
            }),
            (
                "missing credit output over the cap",
                proof(over_cap(), 5),
                |error| {
                    matches!(
                        error,
                        BasicError::IdentityAssetLockTransactionTooManyInputsError(_)
                    )
                },
            ),
            (
                "missing credit output at the cap",
                proof(at_cap(), 5),
                |error| {
                    matches!(
                        error,
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(_)
                    )
                },
            ),
            (
                "credit output not P2PKH at the cap",
                proof(not_p2pkh, 0),
                |error| {
                    matches!(
                        error,
                        BasicError::InvalidIdentityAssetLockTransactionOutputError(_)
                    )
                },
            ),
        ];

        for (case, proof, is_expected) in cases {
            let at_14 = proof
                .validate_structure(protocol_14())
                .expect("a known validator");
            let at_latest = proof
                .validate_structure(PlatformVersion::latest())
                .expect("a known validator");
            assert!(
                matches!(at_14.errors.as_slice(), [ConsensusError::BasicError(error)] if is_expected(error)),
                "{case}: got {:?}",
                at_14.errors
            );
            assert_eq!(at_14.errors, at_latest.errors, "{case}");
        }
    }
}
