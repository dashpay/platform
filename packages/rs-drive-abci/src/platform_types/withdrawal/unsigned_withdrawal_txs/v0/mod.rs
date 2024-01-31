//! Withdrawal transactions definitions and processing
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::AssetUnlockPayload;
use dpp::dashcore::{Transaction, Txid, VarInt};
use std::fmt::Display;
use tenderdash_abci::proto::types::VoteExtension;
use tenderdash_abci::proto::{abci::ExtendVoteExtension, types::VoteExtensionType};

type TxIdAndBytes = (Txid, Vec<u8>);

/// Collection of withdrawal transactions processed at some height/round
#[derive(Debug, Default, Clone)]
pub struct UnsignedWithdrawalTxs(Vec<Transaction>);

impl UnsignedWithdrawalTxs {
    pub fn iter(&self) -> std::slice::Iter<Transaction> {
        self.0.iter()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<Transaction> {
        self.0.into_iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_vec(transactions: Vec<Transaction>) -> Self {
        Self(transactions)
    }

    pub fn drain(&mut self) -> UnsignedWithdrawalTxs {
        Self(self.0.drain(..).collect())
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

impl PartialEq<Vec<VoteExtension>> for UnsignedWithdrawalTxs {
    fn eq(&self, other: &Vec<VoteExtension>) -> bool {
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

    fn ne(&self, other: &Vec<VoteExtension>) -> bool {
        !self.eq(other)
    }
}

impl PartialEq<Vec<ExtendVoteExtension>> for UnsignedWithdrawalTxs {
    fn eq(&self, other: &Vec<ExtendVoteExtension>) -> bool {
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

    fn ne(&self, other: &Vec<ExtendVoteExtension>) -> bool {
        !self.eq(other)
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

impl Into<Vec<ExtendVoteExtension>> for UnsignedWithdrawalTxs {
    fn into(self) -> Vec<ExtendVoteExtension> {
        self.0.iter().map(tx_to_extend_vote_extension).collect()
    }
}

fn tx_to_extend_vote_extension(tx: &Transaction) -> ExtendVoteExtension {
    let request_id = make_extend_vote_request_id(&tx);
    let extension = tx.txid().as_byte_array().to_vec();

    ExtendVoteExtension {
        r#type: VoteExtensionType::ThresholdRecoverRaw as i32,
        extension,
        sign_request_id: Some(request_id),
    }
}

pub fn make_extend_vote_request_id(asset_unlock_tx: &Transaction) -> Vec<u8> {
    let asset_unlock_payload: AssetUnlockPayload = asset_unlock_tx
        .clone()
        .special_transaction_payload
        .unwrap()
        .to_asset_unlock_payload()
        .unwrap();

    let mut request_id = vec![];
    const ASSET_UNLOCK_REQUEST_ID_PREFIX: &str = "plwdtx";
    let prefix_len = VarInt(ASSET_UNLOCK_REQUEST_ID_PREFIX.len() as u64);
    let index = asset_unlock_payload.base.index.to_le_bytes();

    prefix_len.consensus_encode(&mut request_id).unwrap();
    request_id.extend_from_slice(ASSET_UNLOCK_REQUEST_ID_PREFIX.as_bytes());
    request_id.extend_from_slice(&index);

    request_id
}
