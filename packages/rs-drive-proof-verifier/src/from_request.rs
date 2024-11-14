//! Conversions between Drive queries and dapi-grpc requests.

use dapi_grpc::platform::v0::{
    self as proto,
    get_contested_resource_vote_state_request::{
        self, get_contested_resource_vote_state_request_v0,
    },
    get_contested_resources_request::{
        self, get_contested_resources_request_v0, GetContestedResourcesRequestV0,
    },
    get_vote_polls_by_end_date_request::{self},
    GetContestedResourceIdentityVotesRequest, GetContestedResourceVoteStateRequest,
    GetContestedResourceVotersForIdentityRequest, GetContestedResourcesRequest,
    GetPrefundedSpecializedBalanceRequest, GetVotePollsByEndDateRequest,
};
use dpp::{
    identifier::Identifier, platform_value::Value,
    voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
};
use drive::query::{
    contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery,
    vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery,
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
    vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery,
    VotePollsByEndDateDriveQuery,
};

use crate::Error;

const BINCODE_CONFIG: dpp::bincode::config::Configuration = dpp::bincode::config::standard();

/// Conver a gRPC request into a query object.
///
/// This trait is implemented on Drive queries that can be created from gRPC requests.
///
/// # Generic Type Parameters
///
/// * `T`: The type of the gRPC request.
pub trait TryFromRequest<T>: Sized {
    /// Create based on some `grpc_request`.
    fn try_from_request(grpc_request: T) -> Result<Self, Error>;

    /// Try to convert the request into a gRPC query.
    fn try_to_request(&self) -> Result<T, Error>;
}

impl TryFromRequest<get_contested_resource_vote_state_request_v0::ResultType>
    for ContestedDocumentVotePollDriveQueryResultType
{
    fn try_from_request(
        grpc_request: get_contested_resource_vote_state_request_v0::ResultType,
    ) -> Result<Self, Error> {
        use get_contested_resource_vote_state_request_v0::ResultType as GrpcResultType;
        use ContestedDocumentVotePollDriveQueryResultType as DriveResultType;

        Ok(match grpc_request {
            GrpcResultType::Documents => DriveResultType::Documents,
            GrpcResultType::DocumentsAndVoteTally => DriveResultType::DocumentsAndVoteTally,
            GrpcResultType::VoteTally => DriveResultType::VoteTally,
        })
    }
    fn try_to_request(
        &self,
    ) -> Result<get_contested_resource_vote_state_request_v0::ResultType, Error> {
        use get_contested_resource_vote_state_request_v0::ResultType as GrpcResultType;
        use ContestedDocumentVotePollDriveQueryResultType as DriveResultType;

        Ok(match self {
            DriveResultType::Documents => GrpcResultType::Documents,
            DriveResultType::DocumentsAndVoteTally => GrpcResultType::DocumentsAndVoteTally,
            DriveResultType::VoteTally => GrpcResultType::VoteTally,
            DriveResultType::SingleDocumentByContender(_) => {
                return Err(Error::RequestError {
                    error: "can not perform a single document by contender query remotely"
                        .to_string(),
                })
            }
        })
    }
}

impl TryFromRequest<GetContestedResourceVoteStateRequest> for ContestedDocumentVotePollDriveQuery {
    fn try_from_request(grpc_request: GetContestedResourceVoteStateRequest) -> Result<Self, Error> {
        let result = match grpc_request.version.ok_or(Error::EmptyVersion)? {
            get_contested_resource_vote_state_request::Version::V0(v) => {
                ContestedDocumentVotePollDriveQuery {
                    limit: v.count.map(|v| v as u16),
                    vote_poll: ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::from_bytes(&v.contract_id).map_err(|e| {
                            Error::RequestError {
                                error: format!("cannot decode contract id: {}", e),
                            }
                        })?,
                        document_type_name: v.document_type_name.clone(),
                        index_name: v.index_name.clone(),
                        index_values: bincode_decode_values(v.index_values.iter())?,
                    },
                    result_type:  match v.result_type() {
                        get_contested_resource_vote_state_request_v0::ResultType::Documents => {
                            ContestedDocumentVotePollDriveQueryResultType::Documents
                        }
                        get_contested_resource_vote_state_request_v0::ResultType::DocumentsAndVoteTally => {
                            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
                        }
                        get_contested_resource_vote_state_request_v0::ResultType::VoteTally => {
                            ContestedDocumentVotePollDriveQueryResultType::VoteTally
                        }
                    },
                    start_at: v
                        .start_at_identifier_info
                        .map(|v| to_bytes32(&v.start_identifier).map(|id| (id, v.start_identifier_included)))
                        .transpose()
                        .map_err(|e| {
                            Error::RequestError {
                                error: format!(
                                "cannot decode start_at: {}",
                                e
                            )}}
                        )?,
                    offset: None, // offset is not supported when we use proofs
                    allow_include_locked_and_abstaining_vote_tally: v
                        .allow_include_locked_and_abstaining_vote_tally,
                }
            }
        };
        Ok(result)
    }

    fn try_to_request(&self) -> Result<GetContestedResourceVoteStateRequest, Error> {
        use proto::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0 as request_v0;
        if self.offset.is_some() {
            return Err(Error::RequestError{error:"ContestedDocumentVotePollDriveQuery.offset field is internal and must be set to None".into()});
        }

        let start_at_identifier_info = self.start_at.map(|v| request_v0::StartAtIdentifierInfo {
            start_identifier: v.0.to_vec(),
            start_identifier_included: v.1,
        });

        use proto::get_contested_resource_vote_state_request:: get_contested_resource_vote_state_request_v0::ResultType as GrpcResultType;
        Ok(proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
            prove:true,
            contract_id:self.vote_poll.contract_id.to_vec(),
            count: self.limit.map(|v| v as u32),
            document_type_name: self.vote_poll.document_type_name.clone(),
            index_name: self.vote_poll.index_name.clone(),
            index_values: self.vote_poll.index_values.iter().map(|v|
                dpp::bincode::encode_to_vec(v, BINCODE_CONFIG).map_err(|e|Error::RequestError { error: e.to_string() } )).collect::<Result<Vec<_>,_>>()?,
            result_type:match self.result_type {
                ContestedDocumentVotePollDriveQueryResultType::Documents => GrpcResultType::Documents.into(),
                ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally => GrpcResultType::DocumentsAndVoteTally.into(),
                ContestedDocumentVotePollDriveQueryResultType::VoteTally => GrpcResultType::VoteTally.into(),
                ContestedDocumentVotePollDriveQueryResultType::SingleDocumentByContender(_) => return Err(Error::RequestError {
                                                                                                                                                                           error: "can not perform a single document by contender query remotely".to_string(),
                                                                                                                                                                       }),
            },
            start_at_identifier_info,
            allow_include_locked_and_abstaining_vote_tally: self.allow_include_locked_and_abstaining_vote_tally,
        }
        .into())
    }
}

fn to_bytes32(v: &[u8]) -> Result<[u8; 32], Error> {
    let result: Result<[u8; 32], std::array::TryFromSliceError> = v.try_into();
    match result {
        Ok(id) => Ok(id),
        Err(e) => Err(Error::RequestError {
            error: format!("cannot decode id: {}", e),
        }),
    }
}

impl TryFromRequest<GetContestedResourceIdentityVotesRequest>
    for ContestedResourceVotesGivenByIdentityQuery
{
    fn try_from_request(
        grpc_request: GetContestedResourceIdentityVotesRequest,
    ) -> Result<Self, Error> {
        let proto::get_contested_resource_identity_votes_request::Version::V0(value) =
            grpc_request.version.ok_or(Error::EmptyVersion)?;
        let start_at = value
            .start_at_vote_poll_id_info
            .map(|v| {
                to_bytes32(&v.start_at_poll_identifier)
                    .map(|id| (id, v.start_poll_identifier_included))
            })
            .transpose()?;

        Ok(Self {
            identity_id: Identifier::from_vec(value.identity_id.to_vec()).map_err(|e| {
                Error::RequestError {
                    error: e.to_string(),
                }
            })?,
            offset: None,
            limit: value.limit.map(|x| x as u16),
            start_at,
            order_ascending: value.order_ascending,
        })
    }

    fn try_to_request(&self) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        use proto::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0 as request_v0;
        if self.offset.is_some() {
            return Err(Error::RequestError{error:"ContestedResourceVotesGivenByIdentityQuery.offset field is internal and must be set to None".into()});
        }

        Ok(proto::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0 {
                    prove: true,
                    identity_id: self.identity_id.to_vec(),
                    offset: self.offset.map(|x| x as u32),
                    limit: self.limit.map(|x| x as u32),
                    start_at_vote_poll_id_info: self.start_at.map(|(id, included)| {
                        request_v0::StartAtVotePollIdInfo {
                            start_at_poll_identifier: id.to_vec(),
                            start_poll_identifier_included: included,
                        }
                    }),
                    order_ascending: self.order_ascending,
                }.into()
            )
    }
}

use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request;

impl TryFromRequest<GetContestedResourceVotersForIdentityRequest>
    for ContestedDocumentVotePollVotesDriveQuery
{
    fn try_from_request(
        value: GetContestedResourceVotersForIdentityRequest,
    ) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            get_contested_resource_voters_for_identity_request::Version::V0(v) => {
                ContestedDocumentVotePollVotesDriveQuery {
                    vote_poll: ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::from_bytes(&v.contract_id).map_err(|e| {
                            Error::RequestError {
                                error: format!("cannot decode contract id: {}", e),
                            }
                        })?,
                        document_type_name: v.document_type_name.clone(),
                        index_name: v.index_name.clone(),
                        index_values: bincode_decode_values(v.index_values.iter())?,
                    },
                    contestant_id: Identifier::from_bytes(&v.contestant_id).map_err(|e| {
                        Error::RequestError {
                            error: format!("cannot decode contestant_id: {}", e),
                        }
                    })?,
                    limit: v.count.map(|v| v as u16),
                    offset: None,
                    start_at: v
                        .start_at_identifier_info
                        .map(|v| {
                            to_bytes32(&v.start_identifier)
                                .map(|id| (id, v.start_identifier_included))
                        })
                        .transpose()
                        .map_err(|e| Error::RequestError {
                            error: format!("cannot decode start_at value: {}", e),
                        })?,
                    order_ascending: v.order_ascending,
                }
            }
        };

        Ok(result)
    }
    fn try_to_request(&self) -> Result<GetContestedResourceVotersForIdentityRequest, Error> {
        use proto::get_contested_resource_voters_for_identity_request::get_contested_resource_voters_for_identity_request_v0 as request_v0;
        if self.offset.is_some() {
            return Err(Error::RequestError{error:"ContestedDocumentVotePollVotesDriveQuery.offset field is internal and must be set to None".into()});
        }

        Ok(proto::get_contested_resource_voters_for_identity_request::GetContestedResourceVotersForIdentityRequestV0 {
            prove:true,
            contract_id: self.vote_poll.contract_id.to_vec(),
            document_type_name: self.vote_poll.document_type_name.clone(),
            index_name: self.vote_poll.index_name.clone(),
            index_values: self.vote_poll.index_values.iter().map(|v|
                dpp::bincode::encode_to_vec(v, BINCODE_CONFIG).map_err(|e|
                    Error::RequestError { error: e.to_string()})).collect::<Result<Vec<_>,_>>()?,
            order_ascending: self.order_ascending,
            count: self.limit.map(|v| v as u32),
            contestant_id: self.contestant_id.to_vec(),
            start_at_identifier_info: self.start_at.map(|v| request_v0::StartAtIdentifierInfo{
                start_identifier: v.0.to_vec(),
                start_identifier_included: v.1,
            }),
        }
        .into())
    }
}

impl TryFromRequest<GetContestedResourcesRequest> for VotePollsByDocumentTypeQuery {
    fn try_from_request(value: GetContestedResourcesRequest) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            get_contested_resources_request::Version::V0(req) => VotePollsByDocumentTypeQuery {
                contract_id: Identifier::from_bytes(&req.contract_id).map_err(|e| {
                    Error::RequestError {
                        error: format!("cannot decode contract id: {}", e),
                    }
                })?,
                document_type_name: req.document_type_name.clone(),
                index_name: req.index_name.clone(),
                start_at_value: req
                    .start_at_value_info
                    .map(|i| {
                        let (value, _): (Value, _) =
                            bincode::decode_from_slice(&i.start_value, BINCODE_CONFIG).map_err(
                                |e| Error::RequestError {
                                    error: format!("cannot decode start value: {}", e),
                                },
                            )?;
                        Ok::<_, Error>((value, i.start_value_included))
                    })
                    .transpose()?,
                start_index_values: bincode_decode_values(req.start_index_values.iter())?,
                end_index_values: bincode_decode_values(req.end_index_values.iter())?,
                limit: req.count.map(|v| v as u16),
                order_ascending: req.order_ascending,
            },
        };
        Ok(result)
    }

    fn try_to_request(&self) -> Result<GetContestedResourcesRequest, Error> {
        Ok(GetContestedResourcesRequestV0 {
            prove: true,
            contract_id: self.contract_id.to_vec(),
            count: self.limit.map(|v| v as u32),
            document_type_name: self.document_type_name.clone(),
            end_index_values: bincode_encode_values(&self.end_index_values)?,
            start_index_values: bincode_encode_values(&self.start_index_values)?,
            index_name: self.index_name.clone(),
            order_ascending: self.order_ascending,
            start_at_value_info: self
                .start_at_value
                .as_ref()
                .map(|(start_value, start_value_included)| {
                    Ok::<_, Error>(get_contested_resources_request_v0::StartAtValueInfo {
                        start_value: bincode::encode_to_vec(start_value, BINCODE_CONFIG).map_err(
                            |e| Error::RequestError {
                                error: format!("cannot encode start value: {}", e),
                            },
                        )?,
                        start_value_included: *start_value_included,
                    })
                })
                .transpose()?,
        }
        .into())
    }
}

impl TryFromRequest<GetVotePollsByEndDateRequest> for VotePollsByEndDateDriveQuery {
    fn try_from_request(value: GetVotePollsByEndDateRequest) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            get_vote_polls_by_end_date_request::Version::V0(v) => VotePollsByEndDateDriveQuery {
                start_time: v
                    .start_time_info
                    .map(|v| (v.start_time_ms, v.start_time_included)),
                end_time: v
                    .end_time_info
                    .map(|v| (v.end_time_ms, v.end_time_included)),
                limit: v.limit.map(|v| v as u16),
                offset: v.offset.map(|v| v as u16),
                order_ascending: v.ascending,
            },
        };
        Ok(result)
    }

    fn try_to_request(&self) -> Result<GetVotePollsByEndDateRequest, Error> {
        use proto::get_vote_polls_by_end_date_request::get_vote_polls_by_end_date_request_v0 as request_v0;
        if self.offset.is_some() {
            return Err(Error::RequestError {
                error:
                    "VotePollsByEndDateDriveQuery.offset field is internal and must be set to None"
                        .into(),
            });
        }

        Ok(
            proto::get_vote_polls_by_end_date_request::GetVotePollsByEndDateRequestV0 {
                prove: true,
                start_time_info: self.start_time.map(|(start_time_ms, start_time_included)| {
                    request_v0::StartAtTimeInfo {
                        start_time_ms,
                        start_time_included,
                    }
                }),
                end_time_info: self.end_time.map(|(end_time_ms, end_time_included)| {
                    request_v0::EndAtTimeInfo {
                        end_time_ms,
                        end_time_included,
                    }
                }),
                limit: self.limit.map(|v| v as u32),
                offset: self.offset.map(|v| v as u32),
                ascending: self.order_ascending,
            }
            .into(),
        )
    }
}

impl TryFromRequest<GetPrefundedSpecializedBalanceRequest> for Identifier {
    fn try_to_request(&self) -> Result<GetPrefundedSpecializedBalanceRequest, Error> {
        Ok(
            proto::get_prefunded_specialized_balance_request::GetPrefundedSpecializedBalanceRequestV0 {
                prove:true,
                id: self.to_vec(),
            }.into()
        )
    }

    fn try_from_request(
        grpc_request: GetPrefundedSpecializedBalanceRequest,
    ) -> Result<Self, Error> {
        match grpc_request.version.ok_or(Error::EmptyVersion)? {
            proto::get_prefunded_specialized_balance_request::Version::V0(v) => {
                Identifier::from_bytes(&v.id).map_err(|e| Error::RequestError {
                    error: format!("cannot decode id: {}", e),
                })
            }
        }
    }
}

/// Convert a sequence of byte vectors into a sequence of [values](platform_value::Value).
///
/// Small utility function to decode a sequence of byte vectors into a sequence of [values](platform_value::Value).
fn bincode_decode_values<V: AsRef<[u8]>, T: IntoIterator<Item = V>>(
    values: T,
) -> Result<Vec<Value>, Error> {
    values
        .into_iter()
        .map(|v| {
            dpp::bincode::decode_from_slice(v.as_ref(), BINCODE_CONFIG)
                .map_err(|e| Error::RequestError {
                    error: format!("cannot decode value: {}", e),
                })
                .map(|(v, _)| v)
        })
        .collect()
}

/// Convert a sequence of [values](platform_value::Value) into a sequence of byte vectors.
///
/// Small utility function to encode a sequence of [values](platform_value::Value) into a sequence of byte vectors.
fn bincode_encode_values<'a, T: IntoIterator<Item = &'a Value>>(
    values: T,
) -> Result<Vec<Vec<u8>>, Error> {
    values
        .into_iter()
        .map(|v| {
            dpp::bincode::encode_to_vec(v, BINCODE_CONFIG).map_err(|e| Error::RequestError {
                error: format!("cannot encode value: {}", e),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}
