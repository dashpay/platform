//! Withdrawal transactions definitions and processing

use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::Hash;
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
    pub fn iter(&self) -> std::slice::Iter<Transaction> {
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
        Self(self.0.drain(..).collect())
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
    let extension = tx.txid().as_byte_array().to_vec();

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
