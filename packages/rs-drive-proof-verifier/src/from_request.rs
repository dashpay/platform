//! Conversions between Drive queries and dapi-grpc requests.

use dapi_grpc::platform::v0::{
    get_contested_resource_vote_state_request::{
        self, get_contested_resource_vote_state_request_v0,
    },
    GetContestedResourceVoteStateRequest,
};
use dpp::{
    identifier::Identifier, platform_value::Value,
    voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
};
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};

use crate::Error;

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
            GrpcResultType::IdentityIdsOnly => DriveResultType::IdentityIdsOnly,
            GrpcResultType::VoteTally => DriveResultType::VoteTally,
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
                            Error::RequestDecodeError {
                                error: format!("cannot decode contract id: {}", e),
                            }
                        })?,
                        document_type_name: v.document_type_name.clone(),
                        index_name: v.index_name.clone(),
                        index_values: bincode_decode_values(v.index_values.iter()).map_err(
                            |e| Error::RequestDecodeError {
                                error: format!("cannot decode index_values: {}", e),
                            },
                        )?,
                    },
                    result_type: match v.result_type() {
                        get_contested_resource_vote_state_request_v0::ResultType::Documents => {
                            ContestedDocumentVotePollDriveQueryResultType::Documents
                        }
                        get_contested_resource_vote_state_request_v0::ResultType::DocumentsAndVoteTally => {
                            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
                        }
                        get_contested_resource_vote_state_request_v0::ResultType::IdentityIdsOnly => {
                            ContestedDocumentVotePollDriveQueryResultType::IdentityIdsOnly
                        }
                        get_contested_resource_vote_state_request_v0::ResultType::VoteTally => {
                            ContestedDocumentVotePollDriveQueryResultType::VoteTally
                        }
                    },
                    start_at: v
                        .start_at_identifier_info
                        .map(|v| {
                            let result: Result<[u8; 32], std::array::TryFromSliceError> =
                                v.start_identifier.as_slice().try_into();
                            match result {
                                Ok(id) => Ok((id, v.start_identifier_included)),
                                Err(e) => Err(e.to_string()),
                            }
                        })
                        .transpose()
                        .map_err(|e| {
                            Error::RequestDecodeError {
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
}

/// Convert a sequence of byte vectors into a sequence of [values](platform_value::Value).
///
/// Small utility function to decode a sequence of byte vectors into a sequence of [values](platform_value::Value).
fn bincode_decode_values<V: AsRef<[u8]>, T: Iterator<Item = V>>(
    values: T,
) -> Result<Vec<Value>, bincode::error::DecodeError> {
    values
        .map(|v| {
            dpp::bincode::decode_from_slice(v.as_ref(), bincode::config::standard()).map(|(v, _)| v)
        })
        .collect()
}
