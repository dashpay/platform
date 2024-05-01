use crate::abci::AbciError;
use crate::error::Error;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{BlockHash, ChainLock};
use dpp::platform_value::Bytes32;
use std::fmt;
use tenderdash_abci::proto::abci::{RequestPrepareProposal, RequestProcessProposal};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;
use tenderdash_abci::proto::types::CoreChainLock;
use tenderdash_abci::proto::version::Consensus;

/// The block proposal is the combination of information that a proposer will propose,
/// Or that a validator or full node will process
pub struct BlockProposal<'a> {
    /// Consensus Versions
    pub consensus_versions: Consensus,
    /// Block hash
    pub block_hash: Option<[u8; 32]>,
    /// Block height
    pub height: u64,
    /// Block round
    pub round: u32,
    /// Block time in ms
    pub block_time_ms: u64,
    /// Block height of the core chain
    pub core_chain_locked_height: u32,
    /// Potential update core chain lock
    pub core_chain_lock_update: Option<ChainLock>,
    /// The proposed app version
    pub proposed_app_version: u64,
    /// Block proposer's proTxHash
    pub proposer_pro_tx_hash: [u8; 32],
    /// The validator set quorum hash
    pub validator_set_quorum_hash: [u8; 32],
    /// The raw state transitions inside a block proposal
    pub raw_state_transitions: &'a Vec<Vec<u8>>,
}

impl<'a> fmt::Debug for BlockProposal<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BlockProposal {{")?;
        writeln!(f, "  consensus_versions: {:?},", self.consensus_versions)?;
        writeln!(
            f,
            "  block_hash: {:?},",
            self.block_hash.as_ref().map(hex::encode)
        )?;
        writeln!(f, "  height: {},", self.height)?;
        writeln!(f, "  round: {},", self.round)?;
        writeln!(f, "  block_time_ms: {},", self.block_time_ms)?;
        writeln!(
            f,
            "  core_chain_locked_height: {},",
            self.core_chain_locked_height
        )?;
        writeln!(f, "  proposed_app_version: {},", self.proposed_app_version)?;
        writeln!(
            f,
            "  proposer_pro_tx_hash: \"{}\",",
            hex::encode(self.proposer_pro_tx_hash)
        )?;
        writeln!(
            f,
            "  validator_set_quorum_hash: \"{}\",",
            hex::encode(self.validator_set_quorum_hash)
        )?;
        writeln!(
            f,
            "  raw_state_transitions: [{:?}],",
            self.raw_state_transitions
                .iter()
                .map(hex::encode)
                .collect::<Vec<_>>()
        )?;
        write!(f, "}}")
    }
}

impl<'a> TryFrom<&'a RequestPrepareProposal> for BlockProposal<'a> {
    type Error = Error;

    fn try_from(value: &'a RequestPrepareProposal) -> Result<Self, Self::Error> {
        let RequestPrepareProposal {
            max_tx_bytes: _,
            txs,
            local_last_commit: _,
            misbehavior: _,
            height,
            time,
            next_validators_hash: _,
            round,
            core_chain_locked_height,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = value;
        let consensus_versions = version
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "request is missing version".to_string(),
            ))?
            .clone();
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

        if *height < 0 {
            return Err(AbciError::BadRequest(
                "height is negative in request prepare proposal".to_string(),
            )
            .into());
        }
        if *round < 0 {
            return Err(AbciError::BadRequest(
                "round is negative in request prepare proposal".to_string(),
            )
            .into());
        }
        Ok(Self {
            consensus_versions,
            block_hash: None,
            height: *height as u64,
            round: *round as u32,
            core_chain_locked_height: *core_chain_locked_height,
            core_chain_lock_update: None, //there is no need to verify a chain lock we are proposing
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
            proposed_last_commit: _,
            misbehavior: _,
            hash,
            height,
            round,
            time,
            next_validators_hash: _,
            core_chain_locked_height,
            core_chain_lock_update,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = value;
        let consensus_versions = version
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "process proposal request is missing version".to_string(),
            ))?
            .clone();
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

        let block_hash: [u8; 32] = hash.clone().try_into().map_err(|e| {
            Error::Abci(AbciError::BadRequestDataSize(format!(
                "invalid block hash: {}",
                hex::encode(e)
            )))
        })?;
        if *height < 0 {
            return Err(AbciError::BadRequest(
                "height is negative in request process proposal".to_string(),
            )
            .into());
        }
        if *round < 0 {
            return Err(AbciError::BadRequest(
                "round is negative in request process proposal".to_string(),
            )
            .into());
        }

        let core_chain_lock_update = core_chain_lock_update
            .as_ref()
            .map(|core_chain_lock| {
                let CoreChainLock {
                    core_block_height,
                    core_block_hash,
                    signature,
                } = core_chain_lock;

                let block_hash: Bytes32 = Bytes32::from_vec(core_block_hash.clone())?;

                let signature: [u8; 96] = signature.clone().try_into().map_err(|_| {
                    AbciError::BadRequest("core chain lock signature not 96 bytes".to_string())
                })?;

                Ok::<dpp::dashcore::ChainLock, Error>(ChainLock {
                    block_height: *core_block_height,
                    block_hash: BlockHash::from_byte_array(block_hash.0),
                    signature: BLSSignature::from(signature),
                })
            })
            .transpose()?;
        Ok(Self {
            consensus_versions,
            block_hash: Some(block_hash),
            height: *height as u64,
            round: *round as u32,
            core_chain_locked_height: *core_chain_locked_height,
            core_chain_lock_update,
            proposed_app_version: *proposed_app_version,
            proposer_pro_tx_hash,
            validator_set_quorum_hash,
            block_time_ms,
            raw_state_transitions: txs,
        })
    }
}
