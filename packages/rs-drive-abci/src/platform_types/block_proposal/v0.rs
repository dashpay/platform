use crate::abci::AbciError;
use crate::error::Error;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{BlockHash, ChainLock};
use dpp::platform_value::Bytes32;
use std::fmt;
use tenderdash_abci::proto::{
    abci::{RequestPrepareProposal, RequestProcessProposal},
    types::CoreChainLock,
    version::Consensus,
    ToMillis,
};

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

impl fmt::Debug for BlockProposal<'_> {
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
        let consensus_versions = version.as_ref().ok_or(AbciError::BadRequest(
            "request is missing version".to_string(),
        ))?;
        let block_time_ms = time
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "request is missing block time".to_string(),
            ))?
            .to_millis()
            .map_err(|e| AbciError::BadRequest(format!("invalid block time: {}", e)))?;
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
            consensus_versions: *consensus_versions,
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
        let consensus_versions = version.as_ref().ok_or(AbciError::BadRequest(
            "process proposal request is missing version".to_string(),
        ))?;
        let block_time_ms = time
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "missing proposal time".to_string(),
            )))?
            .to_millis()
            .map_err(|e| {
                Error::Abci(AbciError::BadRequest(format!("invalid block time: {}", e)))
            })?;
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
            consensus_versions: *consensus_versions,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tenderdash_abci::proto::google::protobuf::Timestamp;

    fn valid_timestamp() -> Timestamp {
        Timestamp {
            seconds: 1_700_000,
            nanos: 500_000,
        }
    }

    fn valid_prepare_proposal() -> RequestPrepareProposal {
        RequestPrepareProposal {
            max_tx_bytes: 1024,
            txs: vec![vec![1, 2, 3]],
            local_last_commit: None,
            misbehavior: vec![],
            height: 10,
            time: Some(valid_timestamp()),
            next_validators_hash: vec![0u8; 32],
            round: 1,
            core_chain_locked_height: 500,
            proposer_pro_tx_hash: vec![0xAAu8; 32],
            proposed_app_version: 5,
            version: Some(Consensus { block: 1, app: 2 }),
            quorum_hash: vec![0xBBu8; 32],
        }
    }

    fn valid_process_proposal() -> RequestProcessProposal {
        RequestProcessProposal {
            txs: vec![vec![4, 5, 6]],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: vec![0xCCu8; 32],
            height: 20,
            time: Some(valid_timestamp()),
            next_validators_hash: vec![0u8; 32],
            round: 2,
            core_chain_locked_height: 600,
            core_chain_lock_update: None,
            proposer_pro_tx_hash: vec![0xDDu8; 32],
            proposed_app_version: 7,
            version: Some(Consensus { block: 1, app: 3 }),
            quorum_hash: vec![0xEEu8; 32],
        }
    }

    // ---- BlockProposal from RequestPrepareProposal ----

    #[test]
    fn prepare_proposal_valid_conversion() {
        let req = valid_prepare_proposal();
        let proposal = BlockProposal::try_from(&req).expect("should succeed");

        assert_eq!(proposal.height, 10);
        assert_eq!(proposal.round, 1);
        assert_eq!(proposal.core_chain_locked_height, 500);
        assert_eq!(proposal.proposed_app_version, 5);
        assert_eq!(proposal.proposer_pro_tx_hash, [0xAAu8; 32]);
        assert_eq!(proposal.validator_set_quorum_hash, [0xBBu8; 32]);
        assert!(proposal.block_hash.is_none()); // prepare proposal has no block hash
        assert!(proposal.core_chain_lock_update.is_none()); // always None for prepare
        assert_eq!(proposal.raw_state_transitions.len(), 1);
        assert_eq!(proposal.consensus_versions.block, 1);
        assert_eq!(proposal.consensus_versions.app, 2);
    }

    #[test]
    fn prepare_proposal_missing_version_fails() {
        let mut req = valid_prepare_proposal();
        req.version = None;
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_proposal_missing_time_fails() {
        let mut req = valid_prepare_proposal();
        req.time = None;
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_proposal_invalid_proposer_pro_tx_hash_size_fails() {
        let mut req = valid_prepare_proposal();
        req.proposer_pro_tx_hash = vec![0u8; 31]; // wrong size
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_proposal_invalid_quorum_hash_size_fails() {
        let mut req = valid_prepare_proposal();
        req.quorum_hash = vec![0u8; 33]; // wrong size
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_proposal_empty_txs() {
        let mut req = valid_prepare_proposal();
        req.txs = vec![];
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        assert!(proposal.raw_state_transitions.is_empty());
    }

    // ---- BlockProposal from RequestProcessProposal ----

    #[test]
    fn process_proposal_valid_conversion() {
        let req = valid_process_proposal();
        let proposal = BlockProposal::try_from(&req).expect("should succeed");

        assert_eq!(proposal.height, 20);
        assert_eq!(proposal.round, 2);
        assert_eq!(proposal.core_chain_locked_height, 600);
        assert_eq!(proposal.proposed_app_version, 7);
        assert_eq!(proposal.proposer_pro_tx_hash, [0xDDu8; 32]);
        assert_eq!(proposal.validator_set_quorum_hash, [0xEEu8; 32]);
        assert_eq!(proposal.block_hash, Some([0xCCu8; 32]));
        assert!(proposal.core_chain_lock_update.is_none());
        assert_eq!(proposal.raw_state_transitions.len(), 1);
    }

    #[test]
    fn process_proposal_missing_version_fails() {
        let mut req = valid_process_proposal();
        req.version = None;
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_missing_time_fails() {
        let mut req = valid_process_proposal();
        req.time = None;
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_invalid_proposer_hash_size_fails() {
        let mut req = valid_process_proposal();
        req.proposer_pro_tx_hash = vec![0u8; 10]; // wrong size
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_invalid_quorum_hash_size_fails() {
        let mut req = valid_process_proposal();
        req.quorum_hash = vec![0u8; 64]; // wrong size
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_invalid_block_hash_size_fails() {
        let mut req = valid_process_proposal();
        req.hash = vec![0u8; 16]; // wrong size
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_with_core_chain_lock_update() {
        let mut req = valid_process_proposal();
        req.core_chain_lock_update = Some(CoreChainLock {
            core_block_height: 700,
            core_block_hash: vec![0xFFu8; 32],
            signature: vec![0xABu8; 96],
        });
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        assert!(proposal.core_chain_lock_update.is_some());
        let cl = proposal.core_chain_lock_update.unwrap();
        assert_eq!(cl.block_height, 700);
    }

    #[test]
    fn process_proposal_with_invalid_chain_lock_signature_size_fails() {
        let mut req = valid_process_proposal();
        req.core_chain_lock_update = Some(CoreChainLock {
            core_block_height: 700,
            core_block_hash: vec![0xFFu8; 32],
            signature: vec![0xABu8; 48], // wrong size, should be 96
        });
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    #[test]
    fn process_proposal_with_invalid_chain_lock_hash_size_fails() {
        let mut req = valid_process_proposal();
        req.core_chain_lock_update = Some(CoreChainLock {
            core_block_height: 700,
            core_block_hash: vec![0xFFu8; 16], // wrong size
            signature: vec![0xABu8; 96],
        });
        let result = BlockProposal::try_from(&req);
        assert!(result.is_err());
    }

    // ---- Debug formatting ----

    #[test]
    fn block_proposal_debug_format() {
        let req = valid_prepare_proposal();
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        let debug_str = format!("{:?}", proposal);
        assert!(debug_str.contains("BlockProposal"));
        assert!(debug_str.contains("height: 10"));
        assert!(debug_str.contains("round: 1"));
        assert!(debug_str.contains("core_chain_locked_height: 500"));
    }

    #[test]
    fn block_proposal_debug_with_block_hash() {
        let req = valid_process_proposal();
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        let debug_str = format!("{:?}", proposal);
        assert!(debug_str.contains("block_hash"));
        // block_hash should be hex-encoded
        assert!(debug_str.contains("cccccc"));
    }

    // ---- Block time calculation ----

    #[test]
    fn prepare_proposal_block_time_ms_calculated_correctly() {
        let mut req = valid_prepare_proposal();
        req.time = Some(Timestamp {
            seconds: 1000,
            nanos: 500_000_000, // 500ms
        });
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        assert_eq!(proposal.block_time_ms, 1_000_500);
    }

    #[test]
    fn process_proposal_block_time_ms_calculated_correctly() {
        let mut req = valid_process_proposal();
        req.time = Some(Timestamp {
            seconds: 2000,
            nanos: 250_000_000, // 250ms
        });
        let proposal = BlockProposal::try_from(&req).expect("should succeed");
        assert_eq!(proposal.block_time_ms, 2_000_250);
    }
}
