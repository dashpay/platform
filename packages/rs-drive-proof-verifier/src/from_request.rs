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

/// Convert a gRPC request into a query object.
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;

    // ---------------------------------------------------------------
    // Helper: to_bytes32
    // ---------------------------------------------------------------

    #[test]
    fn test_to_bytes32_valid() {
        let input = [0xABu8; 32];
        let result = to_bytes32(&input).expect("should convert 32-byte slice");
        assert_eq!(result, input);
    }

    #[test]
    fn test_to_bytes32_invalid_length() {
        // Too short
        let short = [0u8; 16];
        assert!(to_bytes32(&short).is_err());

        // Too long
        let long = [0u8; 33];
        assert!(to_bytes32(&long).is_err());

        // Empty
        assert!(to_bytes32(&[]).is_err());
    }

    // ---------------------------------------------------------------
    // Helper: bincode encode/decode roundtrip
    // ---------------------------------------------------------------

    #[test]
    fn test_bincode_encode_decode_roundtrip() {
        let values = vec![
            Value::Text("hello".to_string()),
            Value::U64(42),
            Value::Bool(true),
        ];
        let encoded = bincode_encode_values(&values).expect("encoding should succeed");
        assert_eq!(encoded.len(), 3);

        let decoded = bincode_decode_values(encoded.iter()).expect("decoding should succeed");
        assert_eq!(decoded, values);
    }

    #[test]
    fn test_bincode_decode_empty() {
        let empty: Vec<Vec<u8>> = vec![];
        let result = bincode_decode_values(empty.iter()).expect("empty input should succeed");
        assert!(result.is_empty());
    }

    #[test]
    fn test_bincode_decode_invalid() {
        let garbage = [vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB]];
        let result = bincode_decode_values(garbage.iter());
        assert!(
            result.is_err(),
            "invalid bincode bytes should produce an error"
        );
    }

    // ---------------------------------------------------------------
    // TryFromRequest roundtrip: ContestedDocumentVotePollDriveQueryResultType
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_document_vote_poll_result_type_roundtrip() {
        use get_contested_resource_vote_state_request_v0::ResultType as GrpcResultType;

        let cases = vec![
            (
                GrpcResultType::Documents,
                ContestedDocumentVotePollDriveQueryResultType::Documents,
            ),
            (
                GrpcResultType::VoteTally,
                ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            ),
            (
                GrpcResultType::DocumentsAndVoteTally,
                ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            ),
        ];

        for (grpc_val, expected_drive) in cases {
            // grpc -> drive
            let drive_val =
                ContestedDocumentVotePollDriveQueryResultType::try_from_request(grpc_val)
                    .expect("try_from_request should succeed");
            assert_eq!(drive_val, expected_drive);

            // drive -> grpc
            let back = drive_val
                .try_to_request()
                .expect("try_to_request should succeed");
            assert_eq!(back, grpc_val);
        }
    }

    // ---------------------------------------------------------------
    // TryFromRequest roundtrip: ContestedDocumentVotePollDriveQuery
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_document_vote_poll_query_roundtrip() {
        let contract_id = Identifier::from_bytes(&[1u8; 32]).unwrap();
        let index_values = vec![Value::Text("dash".to_string())];

        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: index_values.clone(),
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            offset: None,
            limit: Some(10),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: true,
        };

        let grpc_request = query
            .try_to_request()
            .expect("try_to_request should succeed");

        let roundtripped = ContestedDocumentVotePollDriveQuery::try_from_request(grpc_request)
            .expect("try_from_request should succeed");

        assert_eq!(
            roundtripped.vote_poll.contract_id,
            query.vote_poll.contract_id
        );
        assert_eq!(
            roundtripped.vote_poll.document_type_name,
            query.vote_poll.document_type_name
        );
        assert_eq!(
            roundtripped.vote_poll.index_name,
            query.vote_poll.index_name
        );
        assert_eq!(
            roundtripped.vote_poll.index_values,
            query.vote_poll.index_values
        );
        assert_eq!(roundtripped.result_type, query.result_type);
        assert_eq!(roundtripped.limit, query.limit);
        assert_eq!(roundtripped.start_at, query.start_at);
        assert_eq!(
            roundtripped.allow_include_locked_and_abstaining_vote_tally,
            query.allow_include_locked_and_abstaining_vote_tally
        );
    }

    // ---------------------------------------------------------------
    // TryFromRequest roundtrip: Identifier <-> GetPrefundedSpecializedBalanceRequest
    // ---------------------------------------------------------------

    #[test]
    fn test_identifier_prefunded_balance_roundtrip() {
        let id = Identifier::from_bytes(&[7u8; 32]).unwrap();

        let grpc_request: GetPrefundedSpecializedBalanceRequest =
            id.try_to_request().expect("try_to_request should succeed");

        let roundtripped =
            Identifier::try_from_request(grpc_request).expect("try_from_request should succeed");

        assert_eq!(roundtripped, id);
    }

    // ---------------------------------------------------------------
    // Error path: SingleDocumentByContender is rejected in try_to_request
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_result_type_rejects_single_document_by_contender() {
        let contender_id = Identifier::from_bytes(&[0xCC; 32]).unwrap();
        let result_type =
            ContestedDocumentVotePollDriveQueryResultType::SingleDocumentByContender(contender_id);

        let result = result_type.try_to_request();
        assert!(
            result.is_err(),
            "SingleDocumentByContender should not be convertible to a gRPC request"
        );

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("single document by contender"),
            "error message should mention 'single document by contender', got: {}",
            err_msg
        );
    }

    // ---------------------------------------------------------------
    // Error path: VotePollsByEndDateDriveQuery rejects offset in try_to_request
    // ---------------------------------------------------------------

    // ---------------------------------------------------------------
    // Error path: ContestedDocumentVotePollDriveQuery try_to_request
    // rejects offset != None
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_document_vote_poll_query_rejects_offset() {
        let contract_id = Identifier::from_bytes(&[2u8; 32]).unwrap();
        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: "d".to_string(),
                index_name: "idx".to_string(),
                index_values: vec![],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::Documents,
            offset: Some(5), // should trigger rejection
            limit: None,
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: false,
        };

        let err = query.try_to_request().unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("offset"),
            "error should mention offset, got: {err_msg}"
        );
    }

    // ---------------------------------------------------------------
    // Error path: ContestedResourceVotesGivenByIdentityQuery try_to_request
    // rejects offset != None
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_resource_votes_given_by_identity_rejects_offset() {
        let id = Identifier::from_bytes(&[3u8; 32]).unwrap();
        let query = ContestedResourceVotesGivenByIdentityQuery {
            identity_id: id,
            offset: Some(10), // should trigger rejection
            limit: None,
            start_at: None,
            order_ascending: true,
        };
        let err = query.try_to_request().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("offset"), "error should mention offset: {msg}");
    }

    #[test]
    fn test_contested_resource_votes_given_by_identity_from_request_bad_identity() {
        // identity_id must be exactly 32 bytes; 10 bytes must fail.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::{
            GetContestedResourceIdentityVotesRequestV0, Version as ReqVersion,
        };
        let request = GetContestedResourceIdentityVotesRequest {
            version: Some(ReqVersion::V0(GetContestedResourceIdentityVotesRequestV0 {
                identity_id: vec![0u8; 10],
                start_at_vote_poll_id_info: None,
                limit: None,
                offset: None,
                order_ascending: true,
                prove: true,
            })),
        };
        let err =
            ContestedResourceVotesGivenByIdentityQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn test_contested_resource_votes_given_by_identity_from_request_bad_start_at() {
        // start_at_poll_identifier must be 32 bytes.
        use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::{
            get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo,
            GetContestedResourceIdentityVotesRequestV0, Version as ReqVersion,
        };
        let request = GetContestedResourceIdentityVotesRequest {
            version: Some(ReqVersion::V0(GetContestedResourceIdentityVotesRequestV0 {
                identity_id: vec![0u8; 32],
                start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
                    start_at_poll_identifier: vec![1u8; 9], // bad length
                    start_poll_identifier_included: true,
                }),
                limit: None,
                offset: None,
                order_ascending: true,
                prove: true,
            })),
        };
        let err =
            ContestedResourceVotesGivenByIdentityQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn test_contested_resource_votes_given_by_identity_missing_version() {
        let request = GetContestedResourceIdentityVotesRequest { version: None };
        let err =
            ContestedResourceVotesGivenByIdentityQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    // ---------------------------------------------------------------
    // ContestedDocumentVotePollVotesDriveQuery tests
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_document_vote_poll_votes_missing_version() {
        let request = GetContestedResourceVotersForIdentityRequest { version: None };
        let err = ContestedDocumentVotePollVotesDriveQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn test_contested_document_vote_poll_votes_from_request_bad_contract_id() {
        use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request::{
            GetContestedResourceVotersForIdentityRequestV0, Version as ReqVersion,
        };
        let request = GetContestedResourceVotersForIdentityRequest {
            version: Some(ReqVersion::V0(
                GetContestedResourceVotersForIdentityRequestV0 {
                    contract_id: vec![0u8; 7], // bad
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    index_values: vec![],
                    contestant_id: vec![0u8; 32],
                    start_at_identifier_info: None,
                    order_ascending: true,
                    count: None,
                    prove: true,
                },
            )),
        };
        let err = ContestedDocumentVotePollVotesDriveQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn test_contested_document_vote_poll_votes_from_request_bad_contestant_id() {
        use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request::{
            GetContestedResourceVotersForIdentityRequestV0, Version as ReqVersion,
        };
        let request = GetContestedResourceVotersForIdentityRequest {
            version: Some(ReqVersion::V0(
                GetContestedResourceVotersForIdentityRequestV0 {
                    contract_id: vec![0u8; 32],
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    index_values: vec![],
                    contestant_id: vec![0u8; 5], // bad
                    start_at_identifier_info: None,
                    order_ascending: true,
                    count: None,
                    prove: true,
                },
            )),
        };
        let err = ContestedDocumentVotePollVotesDriveQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("contestant_id"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn test_contested_document_vote_poll_votes_rejects_offset() {
        let contract_id = Identifier::from_bytes(&[0u8; 32]).unwrap();
        let contestant_id = Identifier::from_bytes(&[1u8; 32]).unwrap();
        let q = ContestedDocumentVotePollVotesDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: "d".to_string(),
                index_name: "i".to_string(),
                index_values: vec![],
            },
            contestant_id,
            limit: None,
            offset: Some(7),
            start_at: None,
            order_ascending: true,
        };
        let err = q.try_to_request().unwrap_err();
        assert!(format!("{err}").contains("offset"));
    }

    // ---------------------------------------------------------------
    // VotePollsByDocumentTypeQuery tests
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_polls_by_document_type_missing_version() {
        let request = GetContestedResourcesRequest { version: None };
        let err = VotePollsByDocumentTypeQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn test_vote_polls_by_document_type_from_request_bad_contract_id() {
        let request = GetContestedResourcesRequest {
            version: Some(get_contested_resources_request::Version::V0(
                GetContestedResourcesRequestV0 {
                    contract_id: vec![0u8; 6],
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    start_at_value_info: None,
                    start_index_values: vec![],
                    end_index_values: vec![],
                    count: None,
                    order_ascending: true,
                    prove: true,
                },
            )),
        };
        let err = VotePollsByDocumentTypeQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn test_vote_polls_by_document_type_from_request_bad_start_value() {
        let request = GetContestedResourcesRequest {
            version: Some(get_contested_resources_request::Version::V0(
                GetContestedResourcesRequestV0 {
                    contract_id: vec![0u8; 32],
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    start_at_value_info: Some(
                        get_contested_resources_request_v0::StartAtValueInfo {
                            start_value: vec![0xFFu8, 0xFE, 0xFD], // not valid bincode
                            start_value_included: true,
                        },
                    ),
                    start_index_values: vec![],
                    end_index_values: vec![],
                    count: None,
                    order_ascending: true,
                    prove: true,
                },
            )),
        };
        let err = VotePollsByDocumentTypeQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("decode start value"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn test_vote_polls_by_document_type_roundtrip_with_start_at_value() {
        let contract_id = Identifier::from_bytes(&[9u8; 32]).unwrap();
        let query = VotePollsByDocumentTypeQuery {
            contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parent".to_string(),
            start_at_value: Some((Value::Text("dash".to_string()), true)),
            start_index_values: vec![Value::Text("a".to_string())],
            end_index_values: vec![Value::Text("z".to_string())],
            limit: Some(20),
            order_ascending: false,
        };

        let grpc = query.try_to_request().expect("try_to_request succeeds");
        let back = VotePollsByDocumentTypeQuery::try_from_request(grpc)
            .expect("try_from_request succeeds");

        assert_eq!(back.contract_id, query.contract_id);
        assert_eq!(back.document_type_name, query.document_type_name);
        assert_eq!(back.index_name, query.index_name);
        assert_eq!(back.start_at_value, query.start_at_value);
        assert_eq!(back.start_index_values, query.start_index_values);
        assert_eq!(back.end_index_values, query.end_index_values);
        assert_eq!(back.limit, query.limit);
        assert_eq!(back.order_ascending, query.order_ascending);
    }

    // ---------------------------------------------------------------
    // VotePollsByEndDateDriveQuery happy-path roundtrip
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_polls_by_end_date_roundtrip() {
        let q = VotePollsByEndDateDriveQuery {
            start_time: Some((1, false)),
            end_time: Some((10_000, true)),
            limit: Some(10),
            offset: None,
            order_ascending: false,
        };
        let grpc = q.try_to_request().expect("try_to_request ok");
        let back =
            VotePollsByEndDateDriveQuery::try_from_request(grpc).expect("try_from_request ok");
        assert_eq!(back.start_time, q.start_time);
        assert_eq!(back.end_time, q.end_time);
        assert_eq!(back.limit, q.limit);
        assert_eq!(back.order_ascending, q.order_ascending);
    }

    #[test]
    fn test_vote_polls_by_end_date_missing_version() {
        let request = GetVotePollsByEndDateRequest { version: None };
        let err = VotePollsByEndDateDriveQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    // ---------------------------------------------------------------
    // Identifier / GetPrefundedSpecializedBalanceRequest error paths
    // ---------------------------------------------------------------

    #[test]
    fn test_identifier_prefunded_balance_missing_version() {
        let request = GetPrefundedSpecializedBalanceRequest { version: None };
        let err = Identifier::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn test_identifier_prefunded_balance_bad_id_length() {
        let request = GetPrefundedSpecializedBalanceRequest {
            version: Some(
                proto::get_prefunded_specialized_balance_request::Version::V0(
                    proto::get_prefunded_specialized_balance_request::GetPrefundedSpecializedBalanceRequestV0 {
                        id: vec![0u8; 10], // bad
                        prove: true,
                    },
                ),
            ),
        };
        let err = Identifier::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("decode id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------
    // ContestedDocumentVotePollDriveQuery error paths
    // ---------------------------------------------------------------

    #[test]
    fn test_contested_document_vote_poll_query_missing_version() {
        let request = GetContestedResourceVoteStateRequest { version: None };
        let err = ContestedDocumentVotePollDriveQuery::try_from_request(request).unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn test_contested_document_vote_poll_query_from_request_bad_contract_id() {
        let request = GetContestedResourceVoteStateRequest {
            version: Some(get_contested_resource_vote_state_request::Version::V0(
                proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                    contract_id: vec![0u8; 9], // bad
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    index_values: vec![],
                    result_type: 0,
                    start_at_identifier_info: None,
                    allow_include_locked_and_abstaining_vote_tally: true,
                    count: None,
                    prove: true,
                },
            )),
        };
        let err = ContestedDocumentVotePollDriveQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("contract id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn test_contested_document_vote_poll_query_from_request_bad_start_at_identifier() {
        let request = GetContestedResourceVoteStateRequest {
            version: Some(get_contested_resource_vote_state_request::Version::V0(
                proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                    contract_id: vec![0u8; 32],
                    document_type_name: "d".to_string(),
                    index_name: "i".to_string(),
                    index_values: vec![],
                    result_type: 0,
                    start_at_identifier_info: Some(
                        get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo {
                            start_identifier: vec![0u8; 10], // bad
                            start_identifier_included: true,
                        },
                    ),
                    allow_include_locked_and_abstaining_vote_tally: true,
                    count: None,
                    prove: true,
                },
            )),
        };
        let err = ContestedDocumentVotePollDriveQuery::try_from_request(request).unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("start_at"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------
    // bincode_encode_values: error path
    // ---------------------------------------------------------------

    #[test]
    fn test_bincode_decode_mixed_valid_and_invalid() {
        let mut encoded_valid = bincode_encode_values(&[Value::Text("x".to_string())]).unwrap();
        // Put a corrupted record after a valid one.
        encoded_valid.push(vec![0xFF, 0xFE, 0xFD]);
        let result = bincode_decode_values(encoded_valid.iter());
        assert!(result.is_err(), "mixed input must fail");
    }

    // ---------------------------------------------------------------
    // Original test below (kept for completeness)
    // ---------------------------------------------------------------

    #[test]
    fn test_vote_polls_by_end_date_rejects_offset() {
        let query = VotePollsByEndDateDriveQuery {
            start_time: Some((1000, true)),
            end_time: Some((2000, false)),
            limit: Some(5),
            offset: Some(10), // This should cause an error
            order_ascending: true,
        };

        let result = query.try_to_request();
        assert!(
            result.is_err(),
            "offset must be None for try_to_request to succeed"
        );

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("offset"),
            "error message should mention 'offset', got: {}",
            err_msg
        );
    }
}
