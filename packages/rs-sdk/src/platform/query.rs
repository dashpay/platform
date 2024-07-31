//! Query trait representing criteria for fetching data from Platform.
//!
//! [Query] trait is used to specify individual objects as well as search criteria for fetching multiple objects from Platform.
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request::GetContestedResourceVotersForIdentityRequestV0;
use dapi_grpc::platform::v0::get_contested_resources_request::GetContestedResourcesRequestV0;
use dapi_grpc::platform::v0::{
    self as proto, get_identity_keys_request, get_identity_keys_request::GetIdentityKeysRequestV0,
    AllKeys, GetContestedResourceVoteStateRequest, GetContestedResourceVotersForIdentityRequest,
    GetContestedResourcesRequest, GetEpochsInfoRequest, GetIdentityKeysRequest,
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeVoteStatusRequest,
    KeyRequestType,
};
use dapi_grpc::platform::v0::{
    GetContestedResourceIdentityVotesRequest, GetPrefundedSpecializedBalanceRequest,
    GetVotePollsByEndDateRequest,
};
use dashcore_rpc::dashcore::{hashes::Hash, ProTxHash};
use dpp::version::PlatformVersionError;
use dpp::{block::epoch::EpochIndex, prelude::Identifier};
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive::query::{DriveDocumentQuery, VotePollsByEndDateDriveQuery};
use drive_proof_verifier::from_request::TryFromRequest;
use rs_dapi_client::transport::TransportRequest;
use std::fmt::Debug;

use crate::{error::Error, platform::document_query::DocumentQuery};

use super::types::epoch::EpochQuery;
/// Default limit of epoch records returned by Platform.
pub const DEFAULT_EPOCH_QUERY_LIMIT: u32 = 100;
/// Default limit of epoch records returned by Platform.
pub const DEFAULT_NODES_VOTING_LIMIT: u32 = 100;
/// Trait implemented by objects that can be used as queries.
///
/// [Query] trait is used to specify criteria for fetching data from Platform.
/// It can be used to specify individual objects as well as search criteria for fetching multiple objects from Platform.
///
/// Some examples of queries include:
///
/// 1. [`Identifier`](crate::platform::Identifier) - fetches an object by its identifier; implemented for
/// [Identity](dpp::prelude::Identity), [DataContract](dpp::prelude::DataContract) and [Document](dpp::document::Document).
/// 2. [`DocumentQuery`] - fetches [Document](dpp::document::Document) based on search
/// conditions; see
/// [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
/// for more details.
///
/// ## Example
///
/// To fetch individual [Identity](dpp::prelude::Identity) object by its [Identifier](crate::platform::Identifier),
/// you just need to create it and use [Fetch](crate::platform::Fetch)
/// or [FetchMany](crate::platform::FetchMany) trait:
///
/// ```rust
/// use dash_sdk::{Sdk, platform::{Query, Identifier, Fetch, Identity}};
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let sdk = Sdk::new_mock();
/// let query = Identifier::new(SOME_IDENTIFIER);
/// let identity = Identity::fetch(&sdk, query);
/// ```
///
/// As [Identifier](crate::platform::Identifier) implements [Query], the `query` variable in the code
/// above can be used as a parameter for [Fetch::fetch()](crate::platform::Fetch::fetch())
/// and [FetchMany::fetch_many()](crate::platform::FetchMany::fetch_many()) methods.
pub trait Query<T: TransportRequest + Mockable>: Send + Debug + Clone {
    /// Converts the current instance into an instance of the `TransportRequest` type.
    ///
    /// This method takes ownership of the instance upon which it's called (hence `self`), and attempts to perform the conversion.
    ///
    /// # Arguments
    ///
    /// * `prove` - Whether to include proofs in the response. Only `true` is supported at the moment.
    ///
    /// # Returns
    /// On success, this method yields an instance of the `TransportRequest` type (`T`).
    /// On failure, it yields an [`Error`].
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during the conversion process.
    /// These are returned as [`Error`] instances.
    fn query(self, prove: bool) -> Result<T, Error>;
}

impl<T> Query<T> for T
where
    T: TransportRequest + Sized + Send + Sync + Clone + Debug,
    T::Response: Send + Sync + Debug,
{
    fn query(self, prove: bool) -> Result<T, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        Ok(self)
    }
}

impl Query<proto::GetDataContractRequest> for Identifier {
    fn query(self, prove: bool) -> Result<proto::GetDataContractRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(proto::GetDataContractRequest {
            version: Some(proto::get_data_contract_request::Version::V0(
                proto::get_data_contract_request::GetDataContractRequestV0 { id, prove },
            )),
        })
    }
}

impl Query<proto::GetDataContractsRequest> for Vec<Identifier> {
    fn query(self, prove: bool) -> Result<proto::GetDataContractsRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let ids = self.into_iter().map(|id| id.to_vec()).collect();
        Ok(proto::GetDataContractsRequest {
            version: Some(proto::get_data_contracts_request::Version::V0(
                proto::get_data_contracts_request::GetDataContractsRequestV0 { ids, prove },
            )),
        })
    }
}

impl Query<proto::GetDataContractHistoryRequest> for LimitQuery<(Identifier, u64)> {
    fn query(self, prove: bool) -> Result<proto::GetDataContractHistoryRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let (id, start_at_ms) = self.query;

        Ok(proto::GetDataContractHistoryRequest {
            version: Some(proto::get_data_contract_history_request::Version::V0(
                proto::get_data_contract_history_request::GetDataContractHistoryRequestV0 {
                    id: id.to_vec(),
                    limit: self.limit,
                    offset: None,
                    start_at_ms,
                    prove,
                },
            )),
        })
    }
}

impl Query<proto::GetIdentityKeysRequest> for Identifier {
    /// Get all keys for an identity with provided identifier.
    fn query(self, prove: bool) -> Result<proto::GetIdentityKeysRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let identity_id = self.to_vec();
        Ok(GetIdentityKeysRequest {
            version: Some(get_identity_keys_request::Version::V0(
                GetIdentityKeysRequestV0 {
                    identity_id,
                    prove,
                    limit: None,
                    offset: None,
                    request_type: Some(KeyRequestType {
                        request: Some(proto::key_request_type::Request::AllKeys(AllKeys {})),
                    }),
                },
            )),
        })
    }
}

impl<'a> Query<DocumentQuery> for DriveDocumentQuery<'a> {
    fn query(self, prove: bool) -> Result<DocumentQuery, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let q: DocumentQuery = (&self).into();
        Ok(q)
    }
}

#[derive(Debug, Clone)]
pub struct QueryStartInfo {
    pub start_key: Vec<u8>,
    pub start_included: bool,
}

/// Wrapper around query that allows to specify limit.
///
/// A query that can be used specify limit when fetching multiple objects from Platform
/// using [`FetchMany`](crate::platform::FetchMany) trait.
///
/// ## Example
///
/// ```rust
/// use dash_sdk::{Sdk, platform::{Query, LimitQuery, Identifier, FetchMany, Identity}};
/// use drive_proof_verifier::types::ExtendedEpochInfos;
/// use dpp::block::extended_epoch_info::ExtendedEpochInfo;
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let sdk = Sdk::new_mock();
/// let query = LimitQuery {
///    query: 1,
///    start_info: None,
///    limit: Some(10),
/// };
/// let epoch = ExtendedEpochInfo::fetch_many(&sdk, query);
/// ```
#[derive(Debug, Clone)]
pub struct LimitQuery<Q> {
    /// Actual query to execute
    pub query: Q,
    /// Start info
    pub start_info: Option<QueryStartInfo>,
    /// Max number of records returned
    pub limit: Option<u32>,
}

impl<Q> From<Q> for LimitQuery<Q> {
    fn from(query: Q) -> Self {
        Self {
            query,
            start_info: None,
            limit: None,
        }
    }
}

impl<E: Into<EpochQuery> + Clone + Debug + Send> Query<GetEpochsInfoRequest> for LimitQuery<E> {
    fn query(self, prove: bool) -> Result<GetEpochsInfoRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let inner: EpochQuery = self.query.into();
        Ok(GetEpochsInfoRequest {
            version: Some(proto::get_epochs_info_request::Version::V0(
                proto::get_epochs_info_request::GetEpochsInfoRequestV0 {
                    prove,
                    start_epoch: inner.start.map(|v| v as u32),
                    count: self.limit.unwrap_or(DEFAULT_EPOCH_QUERY_LIMIT),
                    ascending: inner.ascending,
                },
            )),
        })
    }
}

impl Query<GetEpochsInfoRequest> for EpochIndex {
    fn query(self, prove: bool) -> Result<GetEpochsInfoRequest, Error> {
        LimitQuery {
            query: self,
            start_info: None,
            limit: Some(1),
        }
        .query(prove)
    }
}

impl Query<GetProtocolVersionUpgradeStateRequest> for () {
    fn query(self, prove: bool) -> Result<GetProtocolVersionUpgradeStateRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0 {prove}.into())
    }
}

impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for LimitQuery<Option<ProTxHash>> {
    fn query(self, prove: bool) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0 {
            prove,
            // start_pro_tx_hash == [] means "start from beginning"
            start_pro_tx_hash: self.query.map(|v|v.to_byte_array().to_vec()).unwrap_or_default(),
            count: self.limit.unwrap_or(DEFAULT_NODES_VOTING_LIMIT),
        }
        .into())
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for Option<ProTxHash> {
    fn query(self, prove: bool) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        LimitQuery::from(self).query(prove)
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for ProTxHash {
    fn query(self, prove: bool) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        Some(self).query(prove)
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for LimitQuery<ProTxHash> {
    fn query(self, prove: bool) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        LimitQuery {
            query: Some(self.query),
            start_info: None,
            limit: self.limit,
        }
        .query(prove)
    }
}

impl Query<GetContestedResourcesRequest> for VotePollsByDocumentTypeQuery {
    fn query(self, prove: bool) -> Result<GetContestedResourcesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        self.try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourcesRequest> for LimitQuery<GetContestedResourcesRequest> {
    fn query(self, prove: bool) -> Result<GetContestedResourcesRequest, Error> {
        use proto::get_contested_resources_request::{
            get_contested_resources_request_v0::StartAtValueInfo, Version,
        };
        let query = match self.query.query(prove)?.version {
            Some(Version::V0(v0)) => GetContestedResourcesRequestV0 {
                start_at_value_info: self.start_info.map(|v| StartAtValueInfo {
                    start_value: v.start_key,
                    start_value_included: v.start_included,
                }),
                ..v0
            }
            .into(),
            None => {
                return Err(Error::Protocol(
                    PlatformVersionError::UnknownVersionError(
                        "version not present in request".into(),
                    )
                    .into(),
                ))
            }
        };

        Ok(query)
    }
}

impl Query<GetContestedResourceVoteStateRequest> for ContestedDocumentVotePollDriveQuery {
    fn query(self, prove: bool) -> Result<GetContestedResourceVoteStateRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        if self.offset.is_some() {
            return Err(Error::Generic("ContestedDocumentVotePollDriveQuery.offset field is internal and must be set to None".into()));
        }
        self.try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceVoteStateRequest>
    for LimitQuery<ContestedDocumentVotePollDriveQuery>
{
    fn query(self, prove: bool) -> Result<GetContestedResourceVoteStateRequest, Error> {
        use proto::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let result = match  self.query.query(prove)?.version {
            Some(proto::get_contested_resource_vote_state_request::Version::V0(v0)) =>
                    proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                        start_at_identifier_info: self.start_info.map(|v| StartAtIdentifierInfo {
                            start_identifier: v.start_key,
                            start_identifier_included: v.start_included,
                        }),
                        ..v0
                    }.into(),

            None =>return  Err(Error::Protocol(
                PlatformVersionError::UnknownVersionError("version not present in request".into()).into(),
            )),
        };

        Ok(result)
    }
}

impl Query<GetContestedResourceVotersForIdentityRequest>
    for ContestedDocumentVotePollVotesDriveQuery
{
    fn query(self, prove: bool) -> Result<GetContestedResourceVotersForIdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        if self.offset.is_some() {
            return Err(Error::Generic("ContestedDocumentVotePollVotesDriveQuery.offset field is internal and must be set to None".into()));
        }

        self.try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceVotersForIdentityRequest>
    for LimitQuery<GetContestedResourceVotersForIdentityRequest>
{
    fn query(self, prove: bool) -> Result<GetContestedResourceVotersForIdentityRequest, Error> {
        use proto::get_contested_resource_voters_for_identity_request::{
            get_contested_resource_voters_for_identity_request_v0::StartAtIdentifierInfo, Version,
        };
        let query = match self.query.query(prove)?.version {
            Some(Version::V0(v0)) => GetContestedResourceVotersForIdentityRequestV0 {
                start_at_identifier_info: self.start_info.map(|v| StartAtIdentifierInfo {
                    start_identifier: v.start_key,
                    start_identifier_included: v.start_included,
                }),
                ..v0
            }
            .into(),
            None => {
                return Err(Error::Protocol(
                    PlatformVersionError::UnknownVersionError(
                        "version not present in request".into(),
                    )
                    .into(),
                ))
            }
        };

        Ok(query)
    }
}

impl Query<GetContestedResourceIdentityVotesRequest>
    for ContestedResourceVotesGivenByIdentityQuery
{
    fn query(self, prove: bool) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        if self.offset.is_some() {
            return Err(Error::Generic("ContestedResourceVotesGivenByIdentityQuery.offset field is internal and must be set to None".into()));
        }

        self.try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceIdentityVotesRequest> for ProTxHash {
    fn query(self, prove: bool) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        Ok(GetContestedResourceIdentityVotesRequestV0 {
            identity_id: self.to_byte_array().to_vec(),
            prove,
            limit: None,
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: None,
        }
        .into())
    }
}

impl Query<GetVotePollsByEndDateRequest> for VotePollsByEndDateDriveQuery {
    fn query(self, prove: bool) -> Result<GetVotePollsByEndDateRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        self.try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetPrefundedSpecializedBalanceRequest> for Identifier {
    fn query(self, prove: bool) -> Result<GetPrefundedSpecializedBalanceRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        self.try_to_request().map_err(|e| e.into())
    }
}

/// Query for single vote.
#[derive(Debug, Clone)]
pub struct VoteQuery {
    pub identity_id: Identifier,
    pub vote_poll_id: Identifier,
}
impl VoteQuery {
    pub fn new(identity_id: Identifier, vote_poll_id: Identifier) -> Self {
        Self {
            identity_id,
            vote_poll_id,
        }
    }
}

impl Query<GetContestedResourceIdentityVotesRequest> for VoteQuery {
    fn query(self, prove: bool) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        use proto::get_contested_resource_identity_votes_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo;

        Ok(GetContestedResourceIdentityVotesRequestV0 {
            identity_id: self.identity_id.to_vec(),
            prove,
            limit: Some(1),
            offset: None,
            order_ascending: true,
            start_at_vote_poll_id_info: Some(StartAtVotePollIdInfo {
                start_at_poll_identifier: self.vote_poll_id.to_vec(),
                start_poll_identifier_included: true,
            }),
        }
        .into())
    }
}

impl Query<GetContestedResourceIdentityVotesRequest> for LimitQuery<VoteQuery> {
    fn query(self, prove: bool) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        use proto::get_contested_resource_identity_votes_request::{
            get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo, Version,
        };

        Ok(match self.query.query(prove)?.version {
            None => return Err(Error::Protocol(dpp::ProtocolError::NoProtocolVersionError)),
            Some(Version::V0(v0)) => GetContestedResourceIdentityVotesRequestV0 {
                limit: self.limit,
                start_at_vote_poll_id_info: self.start_info.map(|v| StartAtVotePollIdInfo {
                    start_at_poll_identifier: v.start_key.to_vec(),
                    start_poll_identifier_included: v.start_included,
                }),
                ..v0
            },
        }
        .into())
    }
}
