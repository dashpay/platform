use tenderdash_abci::proto::abci::{RequestPrepareProposal, RequestProcessProposal};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;
use crate::error::Error;

pub struct BlockProposal<'a> {
    proposed_app_version: u64,
    proposer_pro_tx_hash: [u8; 32],
    validator_set_quorum_hash: [u8; 32],
    block_height: u64,
    block_time_ms: u64,
    raw_state_transitions: &'a Vec<Vec<u8>>,
}

impl<'a> TryFrom<&'a RequestPrepareProposal> for BlockProposal<'a> {
    type Error = Error;

    fn try_from(value: &'a RequestPrepareProposal) -> Result<Self, Self::Error> {
        let RequestPrepareProposal {
            max_tx_bytes, txs, local_last_commit, misbehavior, height, time, next_validators_hash, round, core_chain_locked_height, proposer_pro_tx_hash, proposed_app_version, version, quorum_hash
        } = value;
        let block_time_ms = time.as_ref().ok_or("missing proposal time")?.to_milis();
        let proposer_pro_tx_hash: [u8; 32] = proposer_pro_tx_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;
        let validator_set_quorum_hash: [u8; 32] = quorum_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;
        Ok(Self {
            proposed_app_version: *proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_height: *height as u64,
            block_time_ms,
            raw_state_transitions: txs,
        })
    }
}

impl<'a> TryFrom<&'a RequestProcessProposal> for BlockProposal<'a> {
    type Error = Error;

    fn try_from(value: &'a RequestProcessProposal) -> Result<Self, Self::Error> {
        let RequestProcessProposal {
            txs, proposed_last_commit, misbehavior, hash, height, round, time, next_validators_hash, core_chain_locked_height, core_chain_lock_update, proposer_pro_tx_hash, proposed_app_version, version, quorum_hash
        } = value;
        let block_time_ms = time.as_ref().ok_or("missing proposal time")?.to_milis();
        let proposer_pro_tx_hash: [u8; 32] = proposer_pro_tx_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;
        let validator_set_quorum_hash: [u8; 32] = quorum_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;
        Ok(Self {
            proposed_app_version: *proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_height: *height as u64,
            block_time_ms,
            raw_state_transitions: txs,
        })
    }
}