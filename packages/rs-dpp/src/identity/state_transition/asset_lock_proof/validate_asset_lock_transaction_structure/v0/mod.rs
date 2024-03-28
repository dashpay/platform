use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionOutputNotFoundError, InvalidIdentityAssetLockTransactionError,
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
        //Todo: better error
        ConsensusValidationResult::new_with_error(
            InvalidIdentityAssetLockTransactionOutputError::new(output_index as usize).into(),
        )
    } else {
        ConsensusValidationResult::new_with_data(output.clone())
    }
}
