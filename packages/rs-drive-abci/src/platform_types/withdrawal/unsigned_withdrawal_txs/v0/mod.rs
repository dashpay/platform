//! Withdrawal transactions definitions and processing
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dpp::dashcore::{Transaction, VarInt};
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
    /// created based on these transactions
    pub fn are_matching_with_vote_extensions(&self, other: &[VoteExtension]) -> bool {
        if self.0.len() != other.len() {
            return false;
        };

        !self.0.iter().zip(other.iter()).any(|(tx, vote_extension)| {
            let extend_vote_extension = tx_to_extend_vote_extension(tx);

            vote_extension.r#type != extend_vote_extension.r#type
                || vote_extension.sign_request_id != extend_vote_extension.sign_request_id
                || vote_extension.extension != extend_vote_extension.extension
        })
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

fn tx_to_extend_vote_extension(tx: &Transaction) -> ExtendVoteExtension {
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
