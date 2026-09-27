use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use dpp::dashcore::blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::build_asset_unlock_tx;
use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
    AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
};
use dpp::dashcore::{QuorumHash, ScriptBuf, TxOut};

/// Two unsigned withdrawal transactions, with indices 0 and 1, built the way a proposal at
/// chain-locked core height `request_height` builds them.
pub fn unsigned_withdrawal_transactions(request_height: u32) -> UnsignedWithdrawalTxs {
    let transactions = (0..2)
        .map(|index| {
            let untied_transaction = AssetUnlockBaseTransactionInfo {
                version: 1,
                lock_time: 0,
                output: vec![TxOut {
                    value: 100_000,
                    script_pubkey: ScriptBuf::from_bytes(vec![0x51]),
                }],
                base_payload: AssetUnlockBasePayload {
                    version: 1,
                    index,
                    fee: 2_000,
                },
            };

            let mut untied_transaction_bytes = vec![];
            untied_transaction
                .consensus_encode(&mut untied_transaction_bytes)
                .expect("expected to encode an untied withdrawal transaction");

            let request_info = AssetUnlockRequestInfo {
                request_height,
                quorum_hash: QuorumHash::from_byte_array([7u8; 32]),
            };

            let mut unsigned_transaction_bytes = vec![];
            request_info
                .consensus_append_to_base_encode(
                    untied_transaction_bytes,
                    &mut unsigned_transaction_bytes,
                )
                .expect("expected to append the request info");

            build_asset_unlock_tx(&unsigned_transaction_bytes)
                .expect("expected to build an unsigned withdrawal transaction")
        })
        .collect();

    UnsignedWithdrawalTxs::from_vec(transactions)
}
