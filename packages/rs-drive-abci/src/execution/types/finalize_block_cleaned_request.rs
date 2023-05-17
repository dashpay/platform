use crate::abci::AbciError;
use crate::error::Error;
use tenderdash_abci::proto::abci::{CommitInfo, Misbehavior, RequestFinalizeBlock};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{
    Block, BlockId, Commit, CoreChainLock, Data, EvidenceList, Header, PartSetHeader, VoteExtension,
};
use tenderdash_abci::proto::version;

/// The `CleanedCommitInfo` struct represents a `CommitInfo` that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedCommitInfo {
    /// The consensus round number
    pub round: u32,
    /// The hash representing the quorum of validators
    pub quorum_hash: [u8; 32],
    /// The aggregated BLS signature for the block
    pub block_signature: [u8; 96],
    /// The list of additional vote extensions, if any
    pub threshold_vote_extensions: Vec<VoteExtension>,
}

impl TryFrom<CommitInfo> for CleanedCommitInfo {
    type Error = Error;

    fn try_from(value: CommitInfo) -> Result<Self, Self::Error> {
        let CommitInfo {
            round,
            quorum_hash,
            block_signature,
            threshold_vote_extensions,
        } = value;
        if round < 0 {
            return Err(Error::Abci(AbciError::BadRequest(
                "round is negative in commit info".to_string(),
            )));
        }

        let quorum_hash: [u8; 32] = quorum_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "commit info quorum hash is not 32 bytes long".to_string(),
            ))
        })?;

        let block_signature: [u8; 96] = block_signature.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "commit info block signature is not 96 bytes long".to_string(),
            ))
        })?;

        Ok(CleanedCommitInfo {
            round: round as u32,
            quorum_hash,
            block_signature,
            threshold_vote_extensions,
        })
    }
}

/// The `CleanedHeader` struct represents a header that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedHeader {
    /// Basic block info
    pub version: version::Consensus,
    /// The chain id
    pub chain_id: String,
    /// The height of the block
    pub height: u64,
    /// The timestamp of the block
    pub time: Timestamp,

    /// Prev block info
    pub last_block_id: Option<CleanedBlockId>,

    /// Hashes of block data

    /// Commit from validators from the last block
    pub last_commit_hash: Option<[u8; 32]>,

    /// Transactions
    pub data_hash: [u8; 32],

    /// Hashes from the app output from the prev block

    /// Validators for the current block
    pub validators_hash: [u8; 32],

    /// Validators for the next block
    pub next_validators_hash: [u8; 32],

    /// Consensus params for current block
    pub consensus_hash: [u8; 32],

    /// Consensus params for next block
    pub next_consensus_hash: [u8; 32],

    /// State after txs from the previous block
    pub app_hash: [u8; 32],

    /// Root hash of all results from the txs from current block
    pub results_hash: [u8; 32],

    /// Consensus info

    /// Evidence included in the block
    pub evidence_hash: Option<[u8; 32]>,

    /// Proposer's latest available app protocol version
    pub proposed_app_version: u64,

    /// Original proposer of the block
    pub proposer_pro_tx_hash: [u8; 32],

    /// The core chain locked height
    pub core_chain_locked_height: u32,
}

impl TryFrom<Header> for CleanedHeader {
    type Error = Error;

    fn try_from(value: Header) -> Result<Self, Self::Error> {
        let Header {
            version,
            chain_id,
            height,
            time,
            last_block_id,
            last_commit_hash,
            data_hash,
            validators_hash,
            next_validators_hash,
            consensus_hash,
            next_consensus_hash,
            app_hash,
            results_hash,
            evidence_hash,
            proposed_app_version,
            proposer_pro_tx_hash,
            core_chain_locked_height,
        } = value;

        let Some(version) = version else {
            return Err(AbciError::BadRequest(
                "header is missing version".to_string(),
            ).into());
        };

        if height < 0 {
            return Err(
                AbciError::BadRequest("height is negative in block header".to_string()).into(),
            );
        }

        let Some(time) = time else {
            return Err(AbciError::BadRequest(
                "header is missing time".to_string(),
            ).into());
        };

        let last_commit_hash = if last_commit_hash.is_empty() {
            None
        } else {
            Some(last_commit_hash.try_into().map_err(|_| {
                Error::Abci(AbciError::BadRequestDataSize(
                    "header last commit hash is not 32 bytes long".to_string(),
                ))
            })?)
        };

        let data_hash = data_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header data hash is not 32 bytes long".to_string(),
            ))
        })?;

        let validators_hash = validators_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header validators hash is not 32 bytes long".to_string(),
            ))
        })?;

        let next_validators_hash = next_validators_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header next validators hash is not 32 bytes long".to_string(),
            ))
        })?;

        let consensus_hash = consensus_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header consensus hash is not 32 bytes long".to_string(),
            ))
        })?;

        let next_consensus_hash = next_consensus_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header next consensus hash is not 32 bytes long".to_string(),
            ))
        })?;

        let app_hash = app_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header app hash is not 32 bytes long".to_string(),
            ))
        })?;

        let results_hash = results_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header results hash is not 32 bytes long".to_string(),
            ))
        })?;

        let evidence_hash = if evidence_hash.is_empty() {
            None
        } else {
            Some(evidence_hash.try_into().map_err(|_| {
                Error::Abci(AbciError::BadRequestDataSize(
                    "header evidence hash hash is not 32 bytes long".to_string(),
                ))
            })?)
        };

        let proposer_pro_tx_hash = proposer_pro_tx_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "header proposer pro tx hash hash is not 32 bytes long".to_string(),
            ))
        })?;

        Ok(CleanedHeader {
            version,
            chain_id,
            height: height as u64,
            time,
            last_block_id: last_block_id
                .map(|last_block_id| last_block_id.try_into())
                .transpose()?,
            last_commit_hash,
            data_hash,
            validators_hash,
            next_validators_hash,
            consensus_hash,
            next_consensus_hash,
            app_hash,
            results_hash,
            evidence_hash,
            proposed_app_version,
            proposer_pro_tx_hash,
            core_chain_locked_height,
        })
    }
}

/// The `CleanedBlockId` struct represents a `blockId` that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedBlockId {
    /// The block id hash
    pub hash: [u8; 32],
    /// The part set header of the block id
    pub part_set_header: PartSetHeader,
    /// The state id
    pub state_id: [u8; 32],
}

impl TryFrom<BlockId> for CleanedBlockId {
    type Error = Error;

    fn try_from(value: BlockId) -> Result<Self, Self::Error> {
        let BlockId {
            hash,
            part_set_header,
            state_id,
        } = value;
        let hash = hash_or_default(hash).map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "hash is not 32 bytes long in block id".to_string(),
            ))
        })?;

        let Some(part_set_header) = part_set_header else {
            return Err(AbciError::BadRequest(
                "block id is missing part set header".to_string(),
            ).into());
        };

        let state_id = hash_or_default(state_id).map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "state id is not 32 bytes long".to_string(),
            ))
        })?;

        Ok(CleanedBlockId {
            hash,
            part_set_header,
            state_id,
        })
    }
}

impl From<CleanedBlockId> for BlockId {
    fn from(value: CleanedBlockId) -> Self {
        Self {
            hash: value.hash.to_vec(),
            part_set_header: Some(value.part_set_header),
            state_id: value.state_id.to_vec(),
        }
    }
}

/// The `CleanedBlock` struct represents a block that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedBlock {
    /// The block header containing metadata about the block, such as its version, height, and hash.
    pub header: CleanedHeader,
    /// The block data containing the actual transactions and other relevant information.
    pub data: Data,
    /// A list of evidence items that may be used for validating or invalidating transactions or other data within the block.
    pub evidence: EvidenceList,
    /// An optional field containing the last commit information, which can be used to verify the consensus of the network on the previous block.
    pub last_commit: Option<Commit>,
    /// An optional field containing the core chain lock information, which can be used to ensure the finality and irreversibility of a block in the chain.
    pub core_chain_lock: Option<CoreChainLock>,
}

impl TryFrom<Block> for CleanedBlock {
    type Error = Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let Block {
            header,
            data,
            evidence,
            last_commit,
            core_chain_lock,
        } = value;
        let Some(header) = header else {
            return Err(AbciError::BadRequest(
                "block is missing header".to_string(),
            ).into());
        };
        let Some(data) = data else {
            return Err(AbciError::BadRequest(
                "block is missing data".to_string(),
            ).into());
        };

        let Some(evidence) = evidence else {
            return Err(AbciError::BadRequest(
                "block is missing evidence".to_string(),
            ).into());
        };

        Ok(CleanedBlock {
            header: header.try_into()?,
            data,
            evidence,
            last_commit,
            core_chain_lock,
        })
    }
}

/// The `FinalizeBlockCleanedRequest` struct represents a `RequestFinalizeBlock` that has been
/// properly formatted.
/// It stores essential data required to finalize the request in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct FinalizeBlockCleanedRequest {
    /// Info about the current commit
    pub commit: CleanedCommitInfo,
    /// List of information about validators that acted incorrectly.
    pub misbehavior: Vec<Misbehavior>,
    /// The block header's hash. Present for convenience (can be derived from the block header).
    pub hash: [u8; 32],
    /// The height of the finalized block.
    pub height: u64,
    /// Round number for the block
    pub round: u32,
    /// The block that was finalized
    pub block: CleanedBlock,
    /// The block ID that was finalized
    pub block_id: CleanedBlockId,
}

impl TryFrom<RequestFinalizeBlock> for FinalizeBlockCleanedRequest {
    type Error = Error;

    fn try_from(value: RequestFinalizeBlock) -> Result<Self, Self::Error> {
        let RequestFinalizeBlock {
            commit,
            misbehavior,
            hash,
            height,
            round,
            block,
            block_id,
        } = value;

        let Some(commit) = commit else {
            return Err(AbciError::BadRequest(
                "finalize block is missing commit".to_string(),
            ).into());
        };

        let Some(block) = block else {
            return Err(AbciError::BadRequest(
                "finalize block is missing actual block".to_string(),
            ).into());
        };

        let Some(block_id) = block_id else {
            return Err(AbciError::BadRequest(
                "finalize block is missing block_id".to_string(),
            ).into());
        };

        let hash = hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "finalize block hash is not 32 bytes long".to_string(),
            ))
        })?;

        if height < 0 {
            return Err(AbciError::BadRequest(
                "height is negative in request prepare proposal".to_string(),
            )
            .into());
        }
        if round < 0 {
            return Err(AbciError::BadRequest(
                "round is negative in request prepare proposal".to_string(),
            )
            .into());
        }

        Ok(FinalizeBlockCleanedRequest {
            commit: commit.try_into()?,
            misbehavior,
            hash,
            height: height as u64,
            round: round as u32,
            block: block.try_into()?,
            block_id: block_id.try_into()?,
        })
    }
}

fn hash_or_default(hash: Vec<u8>) -> Result<[u8; 32], <Vec<u8> as TryInto<[u8; 32]>>::Error> {
    if hash.is_empty() {
        // hash is empty at genesis, we assume it is zeros
        Ok([0u8; 32])
    } else {
        hash.try_into()
    }
}
