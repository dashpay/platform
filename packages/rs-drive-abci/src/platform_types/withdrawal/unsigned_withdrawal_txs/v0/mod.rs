//! Withdrawal transactions definitions and processing

use dpp::dashcore::consensus::{serialize, Encodable};
use dpp::dashcore::hashes::{sha256d, Hash};
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dpp::dashcore::{Transaction, VarInt};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use tenderdash_abci::proto::types::VoteExtension;
use tenderdash_abci::proto::{abci::ExtendVoteExtension, types::VoteExtensionType};

/// Collection of withdrawal transactions processed at some height/round
#[derive(Debug, Default, Clone)]
pub struct UnsignedWithdrawalTxs(Vec<Transaction>);

impl UnsignedWithdrawalTxs {
    /// Returns iterator over borrowed withdrawal transactions
    pub fn iter(&self) -> std::slice::Iter<'_, Transaction> {
        self.0.iter()
    }
    /// Returns a number of withdrawal transactions
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Returns a reference to the first withdrawal transaction
    pub fn first(&self) -> Option<&Transaction> {
        self.0.first()
    }
    /// Returns a reference to the last withdrawal transaction
    pub fn last(&self) -> Option<&Transaction> {
        self.0.last()
    }
    /// Returns true if there are no withdrawal transactions
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Creates a new collection of withdrawal transactions for Vec
    pub fn from_vec(transactions: Vec<Transaction>) -> Self {
        Self(transactions)
    }

    /// Drains all withdrawal transactions from the collection
    pub fn drain(&mut self) -> UnsignedWithdrawalTxs {
        Self(std::mem::take(&mut self.0))
    }

    /// Appends another collection of unsigned withdrawal transactions
    pub fn append(&mut self, mut other: Self) {
        self.0.append(&mut other.0);
    }

    /// Verifies that the collection of unsigned withdrawal transactions matches the given votes extensions
    /// created based on these transactions.
    /// Returns a mapping from transactions to their corresponding vote extensions if they match, or `None` if they don't.
    pub fn verify_and_match_with_vote_extensions<'a>(
        &'a self,
        other: &'a [VoteExtension],
    ) -> Option<BTreeMap<&'a Transaction, &'a VoteExtension>> {
        if self.0.len() != other.len() {
            return None;
        }

        // Build a map from sign_request_id to VoteExtension
        let mut vote_extension_map = HashMap::new();
        for vote_extension in other {
            // Ensure that each signature is 96 bytes (size of a bls sig)
            if vote_extension.signature.len() != 96 {
                return None;
            }
            // Ensure sign_request_id is Some
            if let Some(sign_request_id) = &vote_extension.sign_request_id {
                vote_extension_map.insert(sign_request_id.clone(), vote_extension);
            } else {
                // If sign_request_id is None, we cannot match, return None
                return None;
            }
        }

        let mut tx_to_vote_extension_map = BTreeMap::new();

        // For each transaction, check if a matching vote extension exists
        for tx in &self.0 {
            let extend_vote_extension = tx_to_extend_vote_extension(tx);
            let sign_request_id = match &extend_vote_extension.sign_request_id {
                Some(id) => id,
                None => {
                    // If sign_request_id is None, we cannot match, return None
                    return None;
                }
            };

            match vote_extension_map.get(sign_request_id) {
                Some(vote_extension) => {
                    if vote_extension.r#type != extend_vote_extension.r#type
                        || vote_extension.extension != extend_vote_extension.extension
                    {
                        return None;
                    } else {
                        // All good, insert into map
                        tx_to_vote_extension_map.insert(tx, *vote_extension);
                    }
                }
                None => {
                    // No matching vote extension found
                    return None;
                }
            }
        }

        Some(tx_to_vote_extension_map)
    }
}

impl IntoIterator for UnsignedWithdrawalTxs {
    type Item = Transaction;
    type IntoIter = std::vec::IntoIter<Transaction>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Display for UnsignedWithdrawalTxs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("txs:["))?;
        for tx in &self.0 {
            f.write_fmt(format_args!("{}", tx.txid().to_hex()))?;
        }
        f.write_str("]\n")?;
        Ok(())
    }
}

impl PartialEq<[ExtendVoteExtension]> for UnsignedWithdrawalTxs {
    fn eq(&self, other: &[ExtendVoteExtension]) -> bool {
        if self.0.len() != other.len() {
            return false;
        };

        !self
            .0
            .iter()
            .zip(other.iter())
            .any(|(tx, other_vote_extension)| {
                let self_vote_extension = tx_to_extend_vote_extension(tx);

                &self_vote_extension != other_vote_extension
            })
    }
}

impl From<&UnsignedWithdrawalTxs> for Vec<ExtendVoteExtension> {
    fn from(value: &UnsignedWithdrawalTxs) -> Self {
        value
            .0
            .iter()
            .map(tx_to_extend_vote_extension)
            .collect::<Vec<_>>()
    }
}

pub(crate) fn tx_to_extend_vote_extension(tx: &Transaction) -> ExtendVoteExtension {
    let request_id = make_extend_vote_request_id(tx);
    // `tx` must be unsigned. Core verifies `quorumSig` against the hash of the full serialization
    // with only `quorumSig` zeroed, so the message covers `requestedHeight` and `quorumHash`. For
    // a version 1 asset unlock this hash is the txid; the txid of a version 2 asset unlock leaves
    // those fields out, so it is not the signed message.
    let extension = sha256d::Hash::hash(&serialize(tx)).to_byte_array().to_vec();

    ExtendVoteExtension {
        r#type: VoteExtensionType::ThresholdRecoverRaw as i32,
        extension,
        sign_request_id: Some(request_id),
    }
}

pub(crate) fn make_extend_vote_request_id(asset_unlock_tx: &Transaction) -> Vec<u8> {
    let Some(AssetUnlockPayloadType(ref payload)) = asset_unlock_tx.special_transaction_payload
    else {
        panic!("expected to get AssetUnlockPayloadType");
    };

    let mut request_id = vec![];
    const ASSET_UNLOCK_REQUEST_ID_PREFIX: &str = "plwdtx";
    let prefix_len = VarInt(ASSET_UNLOCK_REQUEST_ID_PREFIX.len() as u64);
    let index = payload.base.index.to_le_bytes();

    prefix_len.consensus_encode(&mut request_id).unwrap();
    request_id.extend_from_slice(ASSET_UNLOCK_REQUEST_ID_PREFIX.as_bytes());
    request_id.extend_from_slice(&index);

    request_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::withdrawals::unsigned_withdrawal_transactions;
    use dpp::dashcore::bls_sig_utils::BLSSignature;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::AssetUnlockPayload;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBasePayload;
    use dpp::dashcore::{PubkeyHash, QuorumHash, ScriptBuf, TxOut};
    use std::str::FromStr;

    /// An unsigned version 2 asset unlock paying 1 Dash to the P2PKH script of `[0x11; 20]`,
    /// the transaction of the DIP-0027 worked examples.
    fn unsigned_version_2_asset_unlock(
        index: u64,
        request_height: u32,
        quorum_hash: [u8; 32],
    ) -> Transaction {
        Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 100_000_000,
                script_pubkey: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x11; 20])),
            }],
            special_transaction_payload: Some(AssetUnlockPayloadType(AssetUnlockPayload {
                base: AssetUnlockBasePayload {
                    version: 2,
                    index,
                    fee: 70_000,
                },
                request_info: AssetUnlockRequestInfo {
                    request_height,
                    quorum_hash: QuorumHash::from_byte_array(quorum_hash),
                },
                quorum_sig: BLSSignature::from([0; 96]),
            })),
        }
    }

    /// Parses a hash displayed in reversed byte order, as Core prints it, into raw bytes.
    fn displayed_hash_bytes(displayed: &str) -> Vec<u8> {
        sha256d::Hash::from_str(displayed)
            .expect("expected a valid hash")
            .to_byte_array()
            .to_vec()
    }

    #[test]
    fn should_sign_the_full_serialization_of_a_version_2_unlock_not_its_txid() {
        // DIP-0027 publishes the version 2 txid of its worked example at index 101: the hash of
        // the full serialization with `requestedHeight`, `quorumHash` and `quorumSig` zeroed.
        let dip_0027_txid = displayed_hash_bytes(
            "3c4db73c8356407a5d7c78df5045bd280f2dc4fd644b06c4bfbdead3d5ae41cf",
        );

        // With the signing info zeroed the message and the txid coincide, which pins the
        // serialization and byte order of the message to Core's.
        let zeroed_signing_info = unsigned_version_2_asset_unlock(101, 0, [0; 32]);
        assert_eq!(
            tx_to_extend_vote_extension(&zeroed_signing_info).extension,
            dip_0027_txid
        );

        // The worked example is requested at height 500 by quorum hash one. Core's message
        // covers both, so it is the hash of the serialization that includes them, not the txid.
        let mut quorum_hash_one = [0; 32];
        quorum_hash_one[0] = 1;
        let worked_example = unsigned_version_2_asset_unlock(101, 500, quorum_hash_one);
        let extension = tx_to_extend_vote_extension(&worked_example).extension;

        assert_ne!(extension, dip_0027_txid);
        assert_eq!(
            extension,
            displayed_hash_bytes(
                "7df5cfea6865226753795ccdd8536781a179f2767febf4281e2236e178dc4a50"
            )
        );
    }

    #[test]
    fn should_sign_a_different_message_when_a_version_2_unlock_is_re_signed() {
        // A re-signed version 2 unlock keeps its txid, but Core verifies the new signature
        // against a message covering the new `requestedHeight` and `quorumHash`. Signing the
        // txid would make every re-signed withdrawal fail verification in Core.
        let first_instance = unsigned_version_2_asset_unlock(101, 500, [1; 32]);
        let re_signed_at_new_height = unsigned_version_2_asset_unlock(101, 700, [1; 32]);
        let re_signed_by_new_quorum = unsigned_version_2_asset_unlock(101, 500, [2; 32]);

        let first_extension = tx_to_extend_vote_extension(&first_instance).extension;

        assert_ne!(
            tx_to_extend_vote_extension(&re_signed_at_new_height).extension,
            first_extension
        );
        assert_ne!(
            tx_to_extend_vote_extension(&re_signed_by_new_quorum).extension,
            first_extension
        );
    }

    #[test]
    fn should_keep_signing_the_txid_of_version_1_unlocks() {
        // The txid of a version 1 unlock is the hash of its full serialization, so for every
        // unlock Platform builds as version 1 the vote extension stays byte for byte the same.
        let transactions = unsigned_withdrawal_transactions(500);

        for transaction in transactions.iter() {
            assert_eq!(
                tx_to_extend_vote_extension(transaction).extension,
                transaction.txid().to_byte_array().to_vec()
            );
        }
    }
}
