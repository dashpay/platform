//! [TryFromVersioned] trait to convert data between different formats, like gRPC and Drive objects.

use dapi_grpc::platform::v0::{
    self as grpc,
    get_contested_resource_vote_state_request::{
        self, get_contested_resource_vote_state_request_v0,
    },
    GetContestedResourceVoteStateRequest, GetContestedResourceVotersForIdentityRequest,
    GetContestedResourcesRequest, GetVotePollsByEndDateRequest,
};
use dpp::bincode::{config::Configuration, error::DecodeError};
use dpp::{
    identifier::Identifier,
    platform_value::Value,
    version::{PlatformVersion, TryFromPlatformVersioned},
    voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll,
};
use drive::query::{
    vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery,
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
    vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery,
    VotePollsByEndDateDriveQuery,
};

use crate::Error;

static BINCODE_CONFIG: Configuration = dpp::bincode::config::standard();

/// TryFromVersioned is an equivalent of [TryFrom] trait for converting data between different formats.
///
/// We don't use [TryFrom] / [TryFromPlatformVersioned] trait directly because it's not possible to implement it for types
/// from external crates, and we don't want to intruduce another dependency between crates like dapi-grpc and drive.
pub trait TryFromVersioned<T>
where
    Self: Sized,
{
    /// Convert the value from T, applying version.
    fn try_from_versioned(value: T, version: &PlatformVersion) -> Result<Self, Error>;
}

impl<T: TryFromPlatformVersioned<Self>> TryFromVersioned<T> for T
where
    T::Error: ToString,
{
    fn try_from_versioned(value: T, version: &PlatformVersion) -> Result<Self, Error> {
        T::try_from_platform_versioned(value, version).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })
    }
}

impl TryFromVersioned<GetContestedResourceVoteStateRequest>
    for ContestedDocumentVotePollDriveQuery
{
    fn try_from_versioned(
        request: GetContestedResourceVoteStateRequest,
        version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let result = match request.version.ok_or(Error::EmptyVersion)? {
            get_contested_resource_vote_state_request::Version::V0(v) => {
                ContestedDocumentVotePollDriveQuery {
                    limit: v.count.map(|v| v as u16),
                    vote_poll: ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::from_bytes(&v.contract_id).map_err(|e| {
                            Error::RequestDecodeError {
                                error: e.to_string(),
                            }
                        })?,
                        document_type_name: v.document_type_name.clone(),
                        index_name: v.index_name.clone(),
                        index_values: bincode_decode_values(v.index_values.iter()).map_err(
                            |e| Error::RequestDecodeError {
                                error: e.to_string(),
                            },
                        )?,
                    },
                    order_ascending: v.order_ascending,
                    result_type: ContestedDocumentVotePollDriveQueryResultType::try_from_versioned(
                        v.result_type(),
                        version,
                    )?,
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
                        .map_err(|e| Error::RequestDecodeError {
                            error: e.to_string(),
                        })?,
                    offset: None, // offset is not supported when we use proofs
                    allow_include_locked_and_abstaining_vote_tally: v
                        .allow_include_locked_and_abstaining_vote_tally,
                }
            }
        };
        Ok(result)
    }
}

impl TryFromVersioned<get_contested_resource_vote_state_request_v0::ResultType>
    for ContestedDocumentVotePollDriveQueryResultType
{
    fn try_from_versioned(
        value: get_contested_resource_vote_state_request_v0::ResultType,
        _version: &PlatformVersion,
    ) -> Result<Self, Error> {
        use get_contested_resource_vote_state_request_v0::ResultType as GrpcResultType;
        use ContestedDocumentVotePollDriveQueryResultType as DriveResultType;

        let result = match value {
            GrpcResultType::Documents => DriveResultType::Documents,
            GrpcResultType::DocumentsAndVoteTally => DriveResultType::DocumentsAndVoteTally,
            GrpcResultType::IdentityIdsOnly => DriveResultType::IdentityIdsOnly,
            GrpcResultType::VoteTally => DriveResultType::VoteTally,
        };
        Ok(result)
    }
}

impl TryFromVersioned<GetVotePollsByEndDateRequest> for VotePollsByEndDateDriveQuery {
    fn try_from_versioned(
        value: GetVotePollsByEndDateRequest,
        _version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            grpc::get_vote_polls_by_end_date_request::Version::V0(v) => {
                VotePollsByEndDateDriveQuery {
                    start_time: v
                        .start_time_info
                        .map(|v| (v.start_time_ms, v.start_time_included)),
                    end_time: v
                        .end_time_info
                        .map(|v| (v.end_time_ms, v.end_time_included)),
                    limit: v.limit.map(|v| v as u16),
                    offset: v.offset.map(|v| v as u16),
                    order_ascending: v.ascending,
                }
            }
        };
        Ok(result)
    }
}

impl TryFromVersioned<GetContestedResourcesRequest> for VotePollsByDocumentTypeQuery {
    fn try_from_versioned(
        value: GetContestedResourcesRequest,
        _version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            grpc::get_contested_resources_request::Version::V0(req) => {
                VotePollsByDocumentTypeQuery {
                    contract_id: Identifier::from_bytes(&req.contract_id).map_err(|e| {
                        Error::RequestDecodeError {
                            error: e.to_string(),
                        }
                    })?,
                    document_type_name: req.document_type_name.clone(),
                    index_name: req.index_name.clone(),
                    start_at_value: req
                        .start_at_value_info
                        .map(|i| (i.start_value, i.start_value_included)),
                    start_index_values: bincode_decode_values(req.start_index_values.iter())
                        .map_err(|e| Error::RequestDecodeError {
                            error: e.to_string(),
                        })?,
                    end_index_values: bincode_decode_values(req.end_index_values.iter()).map_err(
                        |e| Error::RequestDecodeError {
                            error: e.to_string(),
                        },
                    )?,
                    limit: req.count.map(|v| v as u16),
                    order_ascending: req.order_ascending,
                }
            }
        };
        Ok(result)
    }
}

impl TryFromVersioned<GetContestedResourceVotersForIdentityRequest>
    for ContestedDocumentVotePollVotesDriveQuery
{
    fn try_from_versioned(
        value: GetContestedResourceVotersForIdentityRequest,
        _version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let result = match value.version.ok_or(Error::EmptyVersion)? {
            grpc::get_contested_resource_voters_for_identity_request::Version::V0(v) => {
                ContestedDocumentVotePollVotesDriveQuery {
                    vote_poll: ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::from_bytes(&v.contract_id).map_err(|e| {
                            Error::RequestDecodeError {
                                error: e.to_string(),
                            }
                        })?,
                        document_type_name: v.document_type_name.clone(),
                        index_name: v.index_name.clone(),
                        index_values: bincode_decode_values(v.index_values.iter()).map_err(
                            |e| Error::RequestDecodeError {
                                error: e.to_string(),
                            },
                        )?,
                    },
                    contestant_id: Identifier::from_bytes(&v.contestant_id).map_err(|e| {
                        Error::RequestDecodeError {
                            error: e.to_string(),
                        }
                    })?,
                    limit: v.count.map(|v| v as u16),
                    offset: None,
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
                        .map_err(|e| Error::RequestDecodeError {
                            error: e.to_string(),
                        })?,
                    order_ascending: v.order_ascending,
                }
            }
        };

        Ok(result)
    }
}

/// Convert a sequence of byte vectors into a sequence of [values](platform_value::Value).
///
/// Small utility function to decode a sequence of byte vectors into a sequence of [values](platform_value::Value).
pub fn bincode_decode_values<V: AsRef<[u8]>, T: Iterator<Item = V>>(
    values: T,
) -> Result<Vec<Value>, DecodeError> {
    values
        .map(|v| dpp::bincode::decode_from_slice(v.as_ref(), BINCODE_CONFIG).map(|(v, _)| v))
        .collect()
}

/// TryIntoVersioned is an equivalent of [TryInto] trait for converting data between different formats.
pub trait TryIntoVersioned<T> {
    /// Convert the value to T, applying version.
    fn try_into_versioned(self, version: &PlatformVersion) -> Result<T, Error>;
}

impl<T, U> TryIntoVersioned<U> for T
where
    U: TryFromVersioned<T>,
{
    fn try_into_versioned(self, version: &PlatformVersion) -> Result<U, Error> {
        U::try_from_versioned(self, version)
    }
}
