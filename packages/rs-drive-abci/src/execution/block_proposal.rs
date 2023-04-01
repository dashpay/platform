use crate::abci::AbciError;
use crate::error::Error;
use tenderdash_abci::proto::abci::{RequestPrepareProposal, RequestProcessProposal};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

/// The block proposal is the combination of information that a proposer will propose,
/// Or that a validator or full node will process
pub struct BlockProposal<'a> {
    /// Block height
    pub height: u64,
    /// Block round
    pub round: u32,
    /// Block time in ms
    pub block_time_ms: u64,
    /// Block height of the core chain
    pub core_chain_locked_height: u32,
    /// The proposed app version
    pub proposed_app_version: u64,
    /// Block proposer's proTxHash
    pub proposer_pro_tx_hash: [u8; 32],
    /// The validator set quorum hash
    pub validator_set_quorum_hash: [u8; 32],
    /// The raw state transitions inside a block proposal
    pub raw_state_transitions: &'a Vec<Vec<u8>>,
}

impl<'a> TryFrom<&'a RequestPrepareProposal> for BlockProposal<'a> {
    type Error = Error;

    fn try_from(value: &'a RequestPrepareProposal) -> Result<Self, Self::Error> {
        let RequestPrepareProposal {
            max_tx_bytes,
            txs,
            local_last_commit,
            misbehavior,
            height,
            time,
            next_validators_hash,
            round,
            core_chain_locked_height,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = value;
        let block_time_ms = time
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "request is missing block time".to_string(),
            ))?
            .to_milis();
        let proposer_pro_tx_hash: [u8; 32] =
            proposer_pro_tx_hash.clone().try_into().map_err(|e| {
                AbciError::BadRequestDataSize(format!(
                    "invalid proposer proTxHash: {}",
                    hex::encode(e)
                ))
            })?;
        let validator_set_quorum_hash: [u8; 32] = quorum_hash.clone().try_into().map_err(|e| {
            AbciError::BadRequestDataSize(format!(
                "invalid validator set quorumHash: {}",
                hex::encode(e)
            ))
        })?;
        Ok(Self {
            height: *height as u64,
            round: *round as u32,
            core_chain_locked_height: *core_chain_locked_height,
            proposed_app_version: *proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,

            block_time_ms,
            raw_state_transitions: txs,
        })
    }
}

impl<'a> TryFrom<&'a RequestProcessProposal> for BlockProposal<'a> {
    type Error = Error;

    fn try_from(value: &'a RequestProcessProposal) -> Result<Self, Self::Error> {
        let RequestProcessProposal {
            txs,
            proposed_last_commit,
            misbehavior,
            hash,
            height,
            round,
            time,
            next_validators_hash,
            core_chain_locked_height,
            core_chain_lock_update,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = value;
        let block_time_ms = time
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "missing proposal time".to_string(),
            )))?
            .to_milis();
        let proposer_pro_tx_hash: [u8; 32] =
            proposer_pro_tx_hash.clone().try_into().map_err(|e| {
                Error::Abci(AbciError::BadRequestDataSize(format!(
                    "invalid proposer protxhash: {}",
                    hex::encode(e)
                )))
            })?;
        let validator_set_quorum_hash: [u8; 32] = quorum_hash.clone().try_into().map_err(|e| {
            Error::Abci(AbciError::BadRequestDataSize(format!(
                "invalid proposer protxhash: {}",
                hex::encode(e)
            )))
        })?;
        Ok(Self {
            height: *height as u64,
            round: *round as u32,
            core_chain_locked_height: *core_chain_locked_height,
            proposed_app_version: *proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_time_ms,
            raw_state_transitions: txs,
        })
    }
}
