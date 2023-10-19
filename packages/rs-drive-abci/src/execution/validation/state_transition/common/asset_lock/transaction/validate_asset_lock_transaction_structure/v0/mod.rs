use crate::error::Error;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutputNotFoundError, InvalidIdentityAssetLockTransactionError,
    InvalidIdentityAssetLockTransactionOutputError,
};
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::Transaction;
use dpp::validation::SimpleConsensusValidationResult;

/// Validates asset lock transaction structure
pub fn validate_asset_lock_transaction_structure_v0(
    transaction: &Transaction,
    output_index: usize,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();

    // It must be an Asset Lock Special Transaction
    let Some(TransactionPayload::AssetLockPayloadType(ref payload)) =
        transaction.special_transaction_payload
    else {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            InvalidIdentityAssetLockTransactionError::new(
                "Funding transaction must have an Asset Lock Special Transaction Payload",
            )
            .into(),
        ));
    };

    // Output index should point to existing funding output in payload
    let Some(output) = payload.credit_outputs.get(output_index) else {
        result.add_error(IdentityAssetLockTransactionOutputNotFoundError::new(
            output_index,
        ));

        return Ok(result);
    };

    // Output should be P2PKH
    if !output.script_pubkey.is_p2pkh() {
        result.add_error(InvalidIdentityAssetLockTransactionOutputError::new(
            output_index,
        ));

        return Ok(result);
    }

    // TODO: Do we need to perform whole validation what Core supposed to do?

    Ok(result)
}
