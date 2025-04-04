use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::cleaned_abci_messages::cleaned_block_id;

use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::Header;
use tenderdash_abci::proto::version;

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
    pub last_block_id: Option<cleaned_block_id::v0::CleanedBlockId>,

    // Hashes of block data
    /// Commit from validators from the last block
    pub last_commit_hash: Option<[u8; 32]>,

    /// Transactions
    pub data_hash: [u8; 32],

    // Hashes from the app output from the prev block
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

    // Consensus info
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
            return Err(AbciError::BadRequest("header is missing version".to_string()).into());
        };

        if height < 0 {
            return Err(
                AbciError::BadRequest("height is negative in block header".to_string()).into(),
            );
        }

        let Some(time) = time else {
            return Err(AbciError::BadRequest("header is missing time".to_string()).into());
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
