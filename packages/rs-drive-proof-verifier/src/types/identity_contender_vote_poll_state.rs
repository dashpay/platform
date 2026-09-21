//! The state of an identity contender vote poll as a client reads it, proved or not.

use crate::Error;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_request::get_identity_contender_vote_poll_state_request_v0::StartAtIdentifierInfo;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_request::GetIdentityContenderVotePollStateRequestV0;
use dapi_grpc::platform::v0::get_identity_contender_vote_poll_state_response::get_identity_contender_vote_poll_state_response_v0::{
    Contender as ContenderProto, IdentityContenderVotePollState as StateProto, Status as StatusProto,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::{
    IdentityContenderVotePollStatus, IdentityContenderVotePollStoredInfoV0Getters,
};
use drive::query::identity_contender_vote_poll_state_query::{
    IdentityContenderVotePollState as DriveState, IdentityContenderVotePollStateQuery,
};

/// How an identity contender vote poll ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityContenderVotePollFinishedInfo {
    /// The winner; None when the join phase ended with no contender
    pub winner: Option<Identifier>,
    /// Whether the masternodes voted, or a single contender won when the join phase ended
    pub vote_phase_held: bool,
    /// The block the poll resolved in
    pub finished_at: BlockInfo,
}

/// The phase of an identity contender vote poll and when each phase ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityContenderVotePollInfo {
    /// The phase the poll is in
    pub status: IdentityContenderVotePollStatus,
    /// When contenders stop joining
    pub join_end_time_ms: u64,
    /// When the masternodes stop voting, if a vote phase runs
    pub vote_end_time_ms: u64,
    /// How the poll ended, once it did
    pub finished: Option<IdentityContenderVotePollFinishedInfo>,
}

/// One contender of an identity contender vote poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityContender {
    /// The contender's identity
    pub identity_id: Identifier,
    /// The sum of the strengths of the votes towards the contender; None once the poll ended
    pub vote_tally: Option<u32>,
    /// The height of the block the identity joined in
    pub joined_at_block_height: u64,
    /// The time of the block the identity joined in
    pub joined_at_block_time_ms: u64,
    /// The id of what made the identity a contender, the last tie-break key
    pub reference_id: Identifier,
}

/// The state of an identity contender vote poll.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentityContenderVotePollState {
    /// The poll's phase and windows; None for a read that started at a contender
    pub info: Option<IdentityContenderVotePollInfo>,
    /// The contenders in identity id order
    pub contenders: Vec<IdentityContender>,
    /// The tally of abstain votes; None once the poll ended, and for a read that started at a
    /// contender
    pub abstain_vote_tally: Option<u32>,
}

impl IdentityContenderVotePollState {
    /// Whether the read found nothing at all: the poll never opened.
    pub fn is_empty(&self) -> bool {
        self.info.is_none() && self.contenders.is_empty() && self.abstain_vote_tally.is_none()
    }
}

impl From<DriveState> for IdentityContenderVotePollState {
    fn from(state: DriveState) -> Self {
        let DriveState {
            stored_info,
            contenders,
            abstain_vote_tally,
        } = state;
        IdentityContenderVotePollState {
            info: stored_info.map(|stored_info| IdentityContenderVotePollInfo {
                status: stored_info.status(),
                join_end_time_ms: stored_info.join_end_time_ms(),
                vote_end_time_ms: stored_info.vote_end_time_ms(),
                finished: stored_info.result().map(|result| {
                    IdentityContenderVotePollFinishedInfo {
                        winner: result.winner,
                        vote_phase_held: result.vote_phase_held,
                        finished_at: result.finalization_block,
                    }
                }),
            }),
            contenders: contenders
                .into_iter()
                .map(|contender| IdentityContender {
                    identity_id: contender.identity_id,
                    vote_tally: contender.vote_tally,
                    joined_at_block_height: contender.info.joined_at().height,
                    joined_at_block_time_ms: contender.info.joined_at().time_ms,
                    reference_id: contender.info.reference_id(),
                })
                .collect(),
            abstain_vote_tally,
        }
    }
}

/// The Drive query a request asks for, checked the way the node checks it.
pub fn state_query_from_request(
    request: &GetIdentityContenderVotePollStateRequestV0,
    platform_version: &PlatformVersion,
) -> Result<IdentityContenderVotePollStateQuery, Error> {
    let vote_poll_id =
        Identifier::from_bytes(&request.vote_poll_id).map_err(|e| Error::RequestError {
            error: format!("cannot decode vote_poll_id: {}", e),
        })?;
    let max_limit = platform_version.drive_abci.query.max_returned_elements;
    let limit = match request.count {
        None => max_limit,
        Some(count) => {
            let count = u16::try_from(count).unwrap_or(u16::MAX);
            if count == 0 || count > max_limit {
                return Err(Error::RequestError {
                    error: format!("count must be between 1 and {}, got {}", max_limit, count),
                });
            }
            count
        }
    };
    let start_at = request
        .start_at_identifier_info
        .as_ref()
        .map(
            |StartAtIdentifierInfo {
                 start_identifier,
                 start_identifier_included,
             }| {
                Identifier::from_bytes(start_identifier)
                    .map(|identifier| (identifier.to_buffer(), *start_identifier_included))
                    .map_err(|e| Error::RequestError {
                        error: format!("cannot decode start_identifier: {}", e),
                    })
            },
        )
        .transpose()?;
    Ok(IdentityContenderVotePollStateQuery {
        vote_poll_id,
        limit: Some(limit),
        start_at,
    })
}

/// The request a Drive query stands for, with a proof asked for.
pub fn state_request_from_query(
    query: &IdentityContenderVotePollStateQuery,
) -> GetIdentityContenderVotePollStateRequestV0 {
    GetIdentityContenderVotePollStateRequestV0 {
        vote_poll_id: query.vote_poll_id.to_vec(),
        start_at_identifier_info: query.start_at.map(|(start_identifier, included)| {
            StartAtIdentifierInfo {
                start_identifier: start_identifier.to_vec(),
                start_identifier_included: included,
            }
        }),
        count: query.limit.map(u32::from),
        prove: true,
    }
}

fn status_from_proto(status: i32) -> Result<IdentityContenderVotePollStatus, Error> {
    match StatusProto::try_from(status) {
        Ok(StatusProto::Joining) => Ok(IdentityContenderVotePollStatus::Joining),
        Ok(StatusProto::Voting) => Ok(IdentityContenderVotePollStatus::Voting),
        Ok(StatusProto::Resolved) => Ok(IdentityContenderVotePollStatus::Resolved),
        Err(_) => Err(Error::ResponseDecodeError {
            error: format!("unknown identity contender vote poll status {}", status),
        }),
    }
}

fn identifier_from_response(field: &str, bytes: &[u8]) -> Result<Identifier, Error> {
    Identifier::from_bytes(bytes).map_err(|e| Error::ResponseDecodeError {
        error: format!("cannot decode {}: {}", field, e),
    })
}

fn contender_from_proto(contender: ContenderProto) -> Result<IdentityContender, Error> {
    Ok(IdentityContender {
        identity_id: identifier_from_response("identity_id", &contender.identity_id)?,
        vote_tally: contender.vote_tally,
        joined_at_block_height: contender.joined_at_block_height,
        joined_at_block_time_ms: contender.joined_at_block_time_ms,
        reference_id: identifier_from_response("reference_id", &contender.reference_id)?,
    })
}

/// The state an unproved response holds.
pub fn state_from_response(state: StateProto) -> Result<IdentityContenderVotePollState, Error> {
    let StateProto {
        info,
        contenders,
        abstain_vote_tally,
    } = state;
    let info = info
        .map(|info| {
            Ok::<_, Error>(IdentityContenderVotePollInfo {
                status: status_from_proto(info.status)?,
                join_end_time_ms: info.join_end_time_ms,
                vote_end_time_ms: info.vote_end_time_ms,
                finished: info
                    .finished_vote_info
                    .map(|finished| {
                        Ok::<_, Error>(IdentityContenderVotePollFinishedInfo {
                            winner: finished
                                .winner_id
                                .as_deref()
                                .map(|bytes| identifier_from_response("winner_id", bytes))
                                .transpose()?,
                            vote_phase_held: finished.vote_phase_held,
                            finished_at: BlockInfo {
                                time_ms: finished.finished_at_block_time_ms,
                                height: finished.finished_at_block_height,
                                core_height: finished.finished_at_core_block_height,
                                epoch: Epoch::new(
                                    u16::try_from(finished.finished_at_epoch).map_err(|_| {
                                        Error::ResponseDecodeError {
                                            error: format!(
                                                "finished_at_epoch {} is out of bounds",
                                                finished.finished_at_epoch
                                            ),
                                        }
                                    })?,
                                )
                                .map_err(|e| {
                                    Error::ResponseDecodeError {
                                        error: format!("finished_at_epoch is not an epoch: {}", e),
                                    }
                                })?,
                            },
                        })
                    })
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(IdentityContenderVotePollState {
        info,
        contenders: contenders
            .into_iter()
            .map(contender_from_proto)
            .collect::<Result<Vec<_>, Error>>()?,
        abstain_vote_tally,
    })
}
