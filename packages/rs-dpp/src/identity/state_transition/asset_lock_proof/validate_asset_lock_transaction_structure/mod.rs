use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use dashcore::{Transaction, TxOut};
use platform_version::version::PlatformVersion;

mod v0;
mod v1;

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure(
    transaction: &Transaction,
    output_index: u32,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, ProtocolError> {
    match platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .validate_asset_lock_transaction_structure
    {
        0 => Ok(v0::validate_asset_lock_transaction_structure_v0(
            transaction,
            output_index,
            platform_version
                .dpp
                .state_transitions
                .identities
                .asset_locks
                .max_asset_lock_transaction_inputs,
        )),
        1 => Ok(v1::validate_asset_lock_transaction_structure_v1(
            transaction,
            output_index,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_asset_lock_transaction_structure".to_string(),
            known_versions: vec![0, 1],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::tests::fixtures::instant_asset_lock_proof_transaction_fixture_with_inputs;
    use dashcore::transaction::special_transaction::TransactionPayload;

    /// A chain proof's transaction reaches this check from Drive, which fetched it from Core,
    /// so from protocol 15 the instant proof's input cap is not part of it: a mined asset lock
    /// of more than 100 inputs can be redeemed. Protocol 14 keeps refusing it.
    #[test]
    fn should_refuse_a_transaction_over_the_input_cap_only_before_protocol_15() {
        let transaction = instant_asset_lock_proof_transaction_fixture_with_inputs(None, None, 101);

        let protocol_14 = PlatformVersion::get(14).expect("protocol 14 exists");
        let result = validate_asset_lock_transaction_structure(&transaction, 0, protocol_14)
            .expect("protocol 14 selects a known validator");
        match result.errors.as_slice() {
            [ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionTooManyInputsError(error),
            )] => {
                assert_eq!(error.actual_inputs(), 101);
                assert_eq!(error.max_inputs(), 100);
            }
            errors => panic!("expected only the too many inputs error, got {errors:?}"),
        }

        let result =
            validate_asset_lock_transaction_structure(&transaction, 0, PlatformVersion::latest())
                .expect("the latest protocol selects a known validator");
        assert!(result.is_valid(), "got {:?}", result.errors);
        let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &transaction.special_transaction_payload
        else {
            panic!("the fixture is an asset lock");
        };
        assert_eq!(result.data.as_ref(), payload.credit_outputs.first());
    }
}
