//! Withdrawal transactions definitions and processing
use crate::platform_types::withdrawal::signed_withdrawal_txs::v0::SignedWithdrawalTxs;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::{
    build_asset_unlock_tx, AssetUnlockPayload,
};
use dpp::dashcore::{Transaction, Txid, VarInt};
use std::collections::BTreeMap;
use std::fmt::Display;
use tenderdash_abci::proto::{abci::ExtendVoteExtension, types::VoteExtensionType};

type TxIdAndBytes = (Txid, Vec<u8>);

/// Collection of withdrawal transactions processed at some height/round
#[derive(Debug, Default)]
pub struct UnsignedWithdrawalTxs(BTreeMap<Txid, Vec<u8>>);

impl UnsignedWithdrawalTxs {
    pub fn iter(&self) -> std::collections::btree_map::Iter<Txid, Vec<u8>> {
        self.0.iter()
    }
}

impl<'a> Display for UnsignedWithdrawalTxs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("txs:["))?;
        for (tx_id, _) in &self.0 {
            f.write_fmt(format_args!("{}", tx_id.to_hex()))?;
        }
        f.write_str("]\n")?;
        Ok(())
    }
}

impl PartialEq<SignedWithdrawalTxs> for UnsignedWithdrawalTxs {
    fn eq(&self, other: &SignedWithdrawalTxs) -> bool {
        if self.0.len() != other.len() {
            return false;
        };

        !self
            .0
            .iter()
            .zip(other.iter())
            .any(|(tx_id_and_bytes, vote_extension)| {
                let (tx_id, tx_bytes) = tx_id_and_bytes;

                let extend_vote_extension =
                    tx_id_and_bytes_to_extend_vote_extension(tx_id, tx_bytes);

                vote_extension.r#type != extend_vote_extension.r#type
                    || vote_extension.sign_request_id != extend_vote_extension.sign_request_id
                    || vote_extension.extension != extend_vote_extension.extension
            })
    }

    fn ne(&self, other: &SignedWithdrawalTxs) -> bool {
        !self.eq(other)
    }
}

impl From<&UnsignedWithdrawalTxs> for Vec<ExtendVoteExtension> {
    fn from(value: &UnsignedWithdrawalTxs) -> Self {
        value
            .0
            .iter()
            .map(|(tx_id, tx_bytes)| tx_id_and_bytes_to_extend_vote_extension(tx_id, tx_bytes))
            .collect::<Vec<_>>()
    }
}

impl Into<Vec<ExtendVoteExtension>> for UnsignedWithdrawalTxs {
    fn into(self) -> Vec<ExtendVoteExtension> {
        self.0
            .into_iter()
            .map(|(tx_id, tx_bytes)| tx_id_and_bytes_to_extend_vote_extension(&tx_id, &tx_bytes))
            .collect()
    }
}

impl From<BTreeMap<Txid, Vec<u8>>> for UnsignedWithdrawalTxs {
    fn from(value: BTreeMap<Txid, Vec<u8>>) -> Self {
        Self(value)
    }
}

impl From<Vec<Vec<u8>>> for UnsignedWithdrawalTxs {
    fn from(value: Vec<Vec<u8>>) -> Self {
        let map = value
            .into_iter()
            .map(|withdrawal_transaction| {
                (
                    Txid::hash(withdrawal_transaction.as_slice()),
                    withdrawal_transaction,
                )
            })
            .collect();

        Self(map)
    }
}

fn tx_id_and_bytes_to_extend_vote_extension(
    tx_id: &Txid,
    tx_bytes: &Vec<u8>,
) -> ExtendVoteExtension {
    let asset_unlock_tx = build_asset_unlock_tx(tx_bytes).unwrap();
    let request_id = make_extend_vote_request_id(&asset_unlock_tx);
    // let extension = tx_id.as_byte_array().to_vec();
    let extension = asset_unlock_tx.txid().as_byte_array().to_vec();

    ExtendVoteExtension {
        r#type: VoteExtensionType::ThresholdRecoverRaw as i32,
        extension,
        sign_request_id: Some(request_id),
    }
}

fn make_extend_vote_request_id(asset_unlock_tx: &Transaction) -> Vec<u8> {
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
