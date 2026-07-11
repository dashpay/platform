//! Query trait representing criteria for fetching data from Platform.
//!
//! [Query] trait is used to specify individual objects as well as search criteria for fetching multiple objects from Platform.
use super::types::epoch::EpochQuery;
use super::types::evonode::EvoNode;
use crate::error::Error;
use crate::platform::documents::document_history_query::DocumentHistoryQuery;
use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_voters_for_identity_request::GetContestedResourceVotersForIdentityRequestV0;
use dapi_grpc::platform::v0::get_contested_resources_request::GetContestedResourcesRequestV0;
use dapi_grpc::platform::v0::get_current_quorums_info_request::GetCurrentQuorumsInfoRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::get_total_credits_in_platform_request::GetTotalCreditsInPlatformRequestV0;
use dapi_grpc::platform::v0::{
    self as proto, get_address_info_request, get_addresses_infos_request,
    get_addresses_trunk_state_request, get_current_quorums_info_request, get_identity_keys_request,
    get_identity_keys_request::GetIdentityKeysRequestV0, get_path_elements_request,
    get_total_credits_in_platform_request, AllKeys, GetAddressInfoRequest,
    GetAddressesInfosRequest, GetAddressesTrunkStateRequest, GetContestedResourceVoteStateRequest,
    GetContestedResourceVotersForIdentityRequest, GetContestedResourcesRequest,
    GetCurrentQuorumsInfoRequest, GetEpochsInfoRequest, GetEvonodesProposedEpochBlocksByIdsRequest,
    GetEvonodesProposedEpochBlocksByRangeRequest, GetIdentityKeysRequest, GetPathElementsRequest,
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeVoteStatusRequest,
    GetTotalCreditsInPlatformRequest, KeyRequestType,
};
use dapi_grpc::platform::v0::{
    get_most_recent_shielded_anchor_request, get_shielded_anchors_request,
    get_shielded_encrypted_notes_request, get_shielded_notes_count_request,
    get_shielded_nullifiers_request, get_shielded_pool_state_request, get_status_request,
    GetContestedResourceIdentityVotesRequest, GetMostRecentShieldedAnchorRequest,
    GetPrefundedSpecializedBalanceRequest, GetShieldedAnchorsRequest,
    GetShieldedEncryptedNotesRequest, GetShieldedNotesCountRequest, GetShieldedNullifiersRequest,
    GetShieldedPoolStateRequest, GetStatusRequest, GetTokenDirectPurchasePricesRequest,
    GetTokenPerpetualDistributionLastClaimRequest, GetVotePollsByEndDateRequest, SpecificKeys,
};
use dpp::address_funds::PlatformAddress;
use dpp::dashcore_rpc::dashcore::{hashes::Hash, ProTxHash};
use dpp::identity::KeyID;
use dpp::version::PlatformVersionError;
use dpp::{block::epoch::EpochIndex, prelude::Identifier};
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive::query::vote_poll_vote_state_query::ContestedDocumentVotePollDriveQuery;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive::query::{DriveDocumentQuery, VotePollsByEndDateDriveQuery};
use drive_proof_verifier::from_request::TryFromRequest;
use drive_proof_verifier::types::{
    KeysInPath, NoParamQuery, ShieldedEncryptedNotesQuery, ShieldedNullifiersQuery,
};
use rs_dapi_client::transport::TransportRequest;
use std::collections::BTreeSet;
use std::fmt::Debug;

/// Default limit of epoch records returned by Platform.
pub const DEFAULT_EPOCH_QUERY_LIMIT: u32 = 100;
/// Default limit of masternode voting records returned by Platform.
pub const DEFAULT_NODES_VOTING_LIMIT: u32 = 100;

/// Trait implemented by objects that can be used as queries.
///
/// [Query] trait is used to specify criteria for fetching data from Platform.
/// It can be used to specify individual objects as well as search criteria for fetching multiple objects from Platform.
///
/// Some examples of queries include:
///
/// 1. [`Identifier`] - fetches an object by its identifier; implemented for
///    [Identity](dpp::prelude::Identity), [DataContract](dpp::prelude::DataContract) and [Document](dpp::document::Document).
/// 2. [`DocumentQuery`] - fetches [Document](dpp::document::Document) based on search conditions; see
///    [query syntax documentation](https://docs.dash.org/projects/platform/en/stable/docs/reference/query-syntax.html)
///    for more details.
///
/// ## Example
///
/// To fetch individual [Identity](dpp::prelude::Identity) object by its [`Identifier`],
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
/// As [`Identifier`] implements [Query], the `query` variable in the code
/// above can be used as a parameter for [Fetch::fetch()](crate::platform::Fetch::fetch())
/// and [FetchMany::fetch_many()](crate::platform::FetchMany::fetch_many()) methods.
pub trait Query<T: Mockable>: Send + Debug + Clone {
    /// Convert the query into a wire-shape [`TransportRequest`].
    ///
    /// # Arguments
    ///
    /// * `settings` - A [`QuerySettings`](crate::platform::QuerySettings) borrowing the encoder
    ///   inputs from the SDK: protocol version (used by encoders that pick wire shapes
    ///   per version — today only [`DocumentQuery`]'s V0/V1 split), `prove` flag,
    ///   and request settings. Construct from an SDK via
    ///   [`Sdk::query_settings`](crate::Sdk::query_settings), or directly in unit tests
    ///   that want to exercise the encoder without spinning up an `Sdk`.
    ///
    /// # Returns
    /// On success, this method yields an instance of the `TransportRequest` type (`T`).
    /// On failure, it yields an [`Error`].
    fn query(&self, settings: &crate::platform::QuerySettings<'_>) -> Result<T, Error>;
}

impl<T> Query<T> for T
where
    T: TransportRequest + Sized + Send + Sync + Clone + Debug,
    T::Response: Send + Sync + Debug,
{
    fn query(&self, settings: &crate::platform::QuerySettings<'_>) -> Result<T, Error> {
        let prove = settings.prove;
        if !prove {
            tracing::warn!(request= ?self, "sending query without proof, ensure data is trusted");
        }
        Ok(self.clone())
    }
}

impl Query<proto::GetDataContractRequest> for Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetDataContractRequest, Error> {
        let prove = settings.prove;
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetDataContractsRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let ids = self.iter().map(|id| id.to_vec()).collect();
        Ok(proto::GetDataContractsRequest {
            version: Some(proto::get_data_contracts_request::Version::V0(
                proto::get_data_contracts_request::GetDataContractsRequestV0 { ids, prove },
            )),
        })
    }
}

impl Query<proto::GetDataContractHistoryRequest> for LimitQuery<(Identifier, u64)> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetDataContractHistoryRequest, Error> {
        let prove = settings.prove;
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

impl Query<proto::GetDocumentHistoryRequest> for DocumentHistoryQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetDocumentHistoryRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::GetDocumentHistoryRequest {
            version: Some(proto::get_document_history_request::Version::V0(
                proto::get_document_history_request::GetDocumentHistoryRequestV0 {
                    data_contract_id: self.data_contract_id.to_vec(),
                    document_type_name: self.document_type_name.clone(),
                    document_id: self.document_id.to_vec(),
                    limit: self.limit,
                    offset: self.offset,
                    start_at_ms: self.start_at_ms,
                    prove,
                },
            )),
        })
    }
}

impl Query<proto::GetIdentityKeysRequest> for Identifier {
    /// Get all keys for an identity with provided identifier.
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetIdentityKeysRequest, Error> {
        let prove = settings.prove;
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

/// Query for specific identity keys by their IDs
#[derive(Debug, Clone)]
pub struct IdentityKeysQuery {
    /// Identity ID to fetch keys from
    pub identity_id: Identifier,
    /// Specific key IDs to fetch
    pub key_ids: Vec<KeyID>,
    /// Optional limit for the number of keys to return
    pub limit: Option<u32>,
    /// Optional offset for pagination
    pub offset: Option<u32>,
}

impl IdentityKeysQuery {
    /// Create a new query for specific identity keys
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to fetch keys from
    /// * `key_ids` - The specific key IDs to fetch
    ///
    /// # Example
    ///
    /// ```rust
    /// use dash_sdk::platform::{Identifier, IdentityKeysQuery};
    ///
    /// let identity_id = Identifier::new([1; 32]);
    /// let key_ids = vec![0, 1, 2]; // Fetch keys with IDs 0, 1, and 2
    /// let query = IdentityKeysQuery::new(identity_id, key_ids);
    /// ```
    pub fn new(identity_id: Identifier, key_ids: Vec<KeyID>) -> Self {
        Self {
            identity_id,
            key_ids,
            limit: None,
            offset: None,
        }
    }

    /// Set a limit on the number of keys to return
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set an offset for pagination
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl Query<proto::GetIdentityKeysRequest> for IdentityKeysQuery {
    /// Get specific keys for an identity.
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetIdentityKeysRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetIdentityKeysRequest {
            version: Some(get_identity_keys_request::Version::V0(
                GetIdentityKeysRequestV0 {
                    identity_id: self.identity_id.to_vec(),
                    prove,
                    limit: self.limit,
                    offset: self.offset,
                    request_type: Some(KeyRequestType {
                        request: Some(proto::key_request_type::Request::SpecificKeys(
                            SpecificKeys {
                                key_ids: self.key_ids.to_vec(),
                            },
                        )),
                    }),
                },
            )),
        })
    }
}

impl Query<GetAddressInfoRequest> for PlatformAddress {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetAddressInfoRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetAddressInfoRequest {
            version: Some(get_address_info_request::Version::V0(
                get_address_info_request::GetAddressInfoRequestV0 {
                    address: self.to_bytes(),
                    prove,
                },
            )),
        })
    }
}

impl Query<GetAddressesInfosRequest> for BTreeSet<PlatformAddress> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetAddressesInfosRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let addresses = self.iter().map(|address| address.to_bytes()).collect();

        Ok(GetAddressesInfosRequest {
            version: Some(get_addresses_infos_request::Version::V0(
                get_addresses_infos_request::GetAddressesInfosRequestV0 { addresses, prove },
            )),
        })
    }
}

impl Query<GetAddressesTrunkStateRequest> for () {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetAddressesTrunkStateRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetAddressesTrunkStateRequest {
            version: Some(get_addresses_trunk_state_request::Version::V0(
                get_addresses_trunk_state_request::GetAddressesTrunkStateRequestV0 {},
            )),
        })
    }
}

impl Query<DocumentQuery> for DriveDocumentQuery<'_> {
    fn query(&self, settings: &crate::platform::QuerySettings<'_>) -> Result<DocumentQuery, Error> {
        let prove = settings.prove;
        if !prove {
            // dash-sdk only serves proof-verified responses. Raw,
            // unverified gRPC responses are out of scope for the
            // SDK fetch path — callers needing unverified data
            // should talk to DAPI directly via rs-dapi-client.
            return Err(Error::Config(
                "dash-sdk does not support non-proven queries; proof verification is \
                 mandatory on the SDK fetch path"
                    .to_string(),
            ));
        }
        let q: DocumentQuery = self.into();
        Ok(q)
    }
}

// `DocumentQuery` does not implement [`TransportRequest`] (the wire form is
// [`GetDocumentsRequest`]), so the blanket `Query<T> for T` does not apply
// to it. Provide the identity impl explicitly so the SDK fetch trampoline
// can use a [`DocumentQuery`] both as the user-supplied `Q` and as the
// rich `Self::Query` produced by `Q::query(sdk)`.
impl Query<DocumentQuery> for DocumentQuery {
    fn query(&self, settings: &crate::platform::QuerySettings<'_>) -> Result<DocumentQuery, Error> {
        let prove = settings.prove;
        if !prove {
            tracing::warn!(request= ?self, "sending query without proof, ensure data is trusted");
        }
        Ok(self.clone())
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEpochsInfoRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let inner: EpochQuery = self.query.clone().into();
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEpochsInfoRequest, Error> {
        LimitQuery {
            query: *self,
            start_info: None,
            limit: Some(1),
        }
        .query(settings)
    }
}

impl Query<GetProtocolVersionUpgradeStateRequest> for () {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetProtocolVersionUpgradeStateRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::get_protocol_version_upgrade_state_request::GetProtocolVersionUpgradeStateRequestV0 {prove}.into())
    }
}

impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for LimitQuery<Option<ProTxHash>> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0 {
            prove,
            // start_pro_tx_hash == [] means "start from beginning"
            start_pro_tx_hash: self.query.as_ref().map(|v|v.to_byte_array().to_vec()).unwrap_or_default(),
            count: self.limit.unwrap_or(DEFAULT_NODES_VOTING_LIMIT),
        }
        .into())
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for Option<ProTxHash> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        LimitQuery::from(*self).query(settings)
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for ProTxHash {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        Some(*self).query(settings)
    }
}

/// Convenience method that allows direct use of a ProTxHash
impl Query<GetProtocolVersionUpgradeVoteStatusRequest> for LimitQuery<ProTxHash> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusRequest, Error> {
        LimitQuery {
            query: Some(self.query),
            start_info: None,
            limit: self.limit,
        }
        .query(settings)
    }
}

impl Query<GetContestedResourcesRequest> for VotePollsByDocumentTypeQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourcesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        self.clone().try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourcesRequest> for LimitQuery<GetContestedResourcesRequest> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourcesRequest, Error> {
        use proto::get_contested_resources_request::{
            get_contested_resources_request_v0::StartAtValueInfo, Version,
        };
        let query = match self.query.query(settings)?.version {
            Some(Version::V0(v0)) => GetContestedResourcesRequestV0 {
                start_at_value_info: self.start_info.clone().map(|v| StartAtValueInfo {
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceVoteStateRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        if self.offset.is_some() {
            return Err(Error::Generic("ContestedDocumentVotePollDriveQuery.offset field is internal and must be set to None".into()));
        }
        self.clone().try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceVoteStateRequest>
    for LimitQuery<ContestedDocumentVotePollDriveQuery>
{
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceVoteStateRequest, Error> {
        let prove = settings.prove;
        use proto::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let result = match  self.query.query(settings)?.version {
            Some(proto::get_contested_resource_vote_state_request::Version::V0(v0)) =>
                    proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                        start_at_identifier_info: self.start_info.clone().map(|v| StartAtIdentifierInfo {
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceVotersForIdentityRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        if self.offset.is_some() {
            return Err(Error::Generic("ContestedDocumentVotePollVotesDriveQuery.offset field is internal and must be set to None".into()));
        }

        self.clone().try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceVotersForIdentityRequest>
    for LimitQuery<GetContestedResourceVotersForIdentityRequest>
{
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceVotersForIdentityRequest, Error> {
        use proto::get_contested_resource_voters_for_identity_request::{
            get_contested_resource_voters_for_identity_request_v0::StartAtIdentifierInfo, Version,
        };
        let query = match self.query.query(settings)?.version {
            Some(Version::V0(v0)) => GetContestedResourceVotersForIdentityRequestV0 {
                start_at_identifier_info: self.start_info.clone().map(|v| StartAtIdentifierInfo {
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

impl Query<GetEvonodesProposedEpochBlocksByRangeRequest>
    for LimitQuery<GetEvonodesProposedEpochBlocksByRangeRequest>
{
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEvonodesProposedEpochBlocksByRangeRequest, Error> {
        use proto::get_evonodes_proposed_epoch_blocks_by_range_request::{
            get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start, Version,
        };
        let query = match self.query.query(settings)?.version {
            Some(Version::V0(v0)) => GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                start: self.start_info.clone().map(|v| {
                    if v.start_included {
                        Start::StartAt(v.start_key)
                    } else {
                        Start::StartAfter(v.start_key)
                    }
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        if self.offset.is_some() {
            return Err(Error::Generic("ContestedResourceVotesGivenByIdentityQuery.offset field is internal and must be set to None".into()));
        }

        self.clone().try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetContestedResourceIdentityVotesRequest> for ProTxHash {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        let prove = settings.prove;
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetVotePollsByEndDateRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        self.clone().try_to_request().map_err(|e| e.into())
    }
}

impl Query<GetPrefundedSpecializedBalanceRequest> for Identifier {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetPrefundedSpecializedBalanceRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        self.clone().try_to_request().map_err(|e| e.into())
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        let prove = settings.prove;
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetContestedResourceIdentityVotesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        use proto::get_contested_resource_identity_votes_request::{
            get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo, Version,
        };

        Ok(match self.query.query(settings)?.version {
            None => return Err(Error::Protocol(dpp::ProtocolError::NoProtocolVersionError)),
            Some(Version::V0(v0)) => GetContestedResourceIdentityVotesRequestV0 {
                limit: self.limit,
                start_at_vote_poll_id_info: self.start_info.clone().map(|v| {
                    StartAtVotePollIdInfo {
                        start_at_poll_identifier: v.start_key.to_vec(),
                        start_poll_identifier_included: v.start_included,
                    }
                }),
                ..v0
            },
        }
        .into())
    }
}

impl Query<GetPathElementsRequest> for KeysInPath {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetPathElementsRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request: GetPathElementsRequest = GetPathElementsRequest {
            version: Some(get_path_elements_request::Version::V0(
                GetPathElementsRequestV0 {
                    path: self.path.clone(),
                    keys: self.keys.clone(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Query<GetTotalCreditsInPlatformRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetTotalCreditsInPlatformRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request: GetTotalCreditsInPlatformRequest = GetTotalCreditsInPlatformRequest {
            version: Some(get_total_credits_in_platform_request::Version::V0(
                GetTotalCreditsInPlatformRequestV0 { prove },
            )),
        };

        Ok(request)
    }
}

impl Query<GetCurrentQuorumsInfoRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetCurrentQuorumsInfoRequest, Error> {
        let prove = settings.prove;
        if prove {
            unimplemented!(
                "query with proof are not supported yet for GetCurrentQuorumsInfoRequest"
            );
        }

        let request: GetCurrentQuorumsInfoRequest = GetCurrentQuorumsInfoRequest {
            version: Some(get_current_quorums_info_request::Version::V0(
                GetCurrentQuorumsInfoRequestV0 {},
            )),
        };

        Ok(request)
    }
}

impl Query<GetEvonodesProposedEpochBlocksByRangeRequest> for LimitQuery<Option<EpochIndex>> {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEvonodesProposedEpochBlocksByRangeRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetEvonodesProposedEpochBlocksByRangeRequest {
            version: Some(proto::get_evonodes_proposed_epoch_blocks_by_range_request::Version::V0(
                GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                    epoch: self.query.map(|v| v as u32),
                    start: self.start_info.clone().map(|v| {
                        use proto::get_evonodes_proposed_epoch_blocks_by_range_request::get_evonodes_proposed_epoch_blocks_by_range_request_v0::Start;
                        if v.start_included {
                            Start::StartAt(v.start_key)
                        } else {
                            Start::StartAfter(v.start_key)
                        }
                    }),
                    limit: self.limit,
                    prove,
                },
            )),
        })
    }
}

impl Query<GetStatusRequest> for EvoNode {
    fn query(&self, _ctx: &crate::platform::QuerySettings<'_>) -> Result<GetStatusRequest, Error> {
        // ignore proof

        let request: GetStatusRequest = GetStatusRequest {
            version: Some(get_status_request::Version::V0(GetStatusRequestV0 {})),
        };

        Ok(request)
    }
}

impl Query<GetTokenDirectPurchasePricesRequest> for &[Identifier] {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetTokenDirectPurchasePricesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request: GetTokenDirectPurchasePricesRequest = GetTokenDirectPurchasePricesRequest {
            version: Some(proto::get_token_direct_purchase_prices_request::Version::V0(
                proto::get_token_direct_purchase_prices_request::GetTokenDirectPurchasePricesRequestV0 {
                    token_ids: self
                        .iter()
                        .map(|identifier| identifier.to_vec())
                        .collect(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

#[derive(Debug, Clone)]
pub struct TokenLastClaimQuery {
    pub token_id: Identifier,
    pub identity_id: Identifier,
}

impl Query<GetTokenPerpetualDistributionLastClaimRequest> for TokenLastClaimQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetTokenPerpetualDistributionLastClaimRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request = GetTokenPerpetualDistributionLastClaimRequest {
            version: Some(
                proto::get_token_perpetual_distribution_last_claim_request::Version::V0(
                    proto::get_token_perpetual_distribution_last_claim_request::GetTokenPerpetualDistributionLastClaimRequestV0 {
                        token_id: self.token_id.to_vec(),
                        identity_id: self.identity_id.to_vec(),
                        contract_info: None, // This field is only used in drive-abci `query_token_perpetual_distribution_last_claim`
                        prove,
                    },
                ),
            ),
        };

        Ok(request)
    }
}

/// Query for fetching proposed block counts by specific evonode IDs
#[derive(Debug, Clone)]
pub struct ProposerBlockCountByIdsQuery {
    /// The epoch to query
    pub epoch: Option<EpochIndex>,
    /// The ProTxHashes to query for
    pub pro_tx_hashes: Vec<ProTxHash>,
}

impl Query<GetEvonodesProposedEpochBlocksByIdsRequest> for ProposerBlockCountByIdsQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEvonodesProposedEpochBlocksByIdsRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        // Convert ProTxHash to bytes
        let ids: Vec<Vec<u8>> = self
            .pro_tx_hashes
            .iter()
            .map(|hash| hash.to_byte_array().to_vec())
            .collect();

        Ok(GetEvonodesProposedEpochBlocksByIdsRequest {
            version: Some(
                proto::get_evonodes_proposed_epoch_blocks_by_ids_request::Version::V0(
                    GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                        epoch: self.epoch.map(|e| e as u32),
                        ids,
                        prove,
                    },
                ),
            ),
        })
    }
}

// Convenience implementation for tuple of (epoch, Vec<ProTxHash>)
impl Query<GetEvonodesProposedEpochBlocksByIdsRequest> for (EpochIndex, Vec<ProTxHash>) {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetEvonodesProposedEpochBlocksByIdsRequest, Error> {
        let (epoch, pro_tx_hashes) = self;
        ProposerBlockCountByIdsQuery {
            epoch: Some(*epoch),
            pro_tx_hashes: pro_tx_hashes.clone(),
        }
        .query(settings)
    }
}

/// Query for fetching recent address balance changes starting from a block height
#[derive(Debug, Clone)]
pub struct RecentAddressBalanceChangesQuery {
    /// The block height to start fetching from
    pub start_height: u64,
}

impl RecentAddressBalanceChangesQuery {
    /// Create a new query starting from a specific block height
    pub fn new(start_height: u64) -> Self {
        Self { start_height }
    }
}

impl Query<proto::GetRecentAddressBalanceChangesRequest> for RecentAddressBalanceChangesQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetRecentAddressBalanceChangesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::GetRecentAddressBalanceChangesRequest {
            version: Some(
                proto::get_recent_address_balance_changes_request::Version::V0(
                    proto::get_recent_address_balance_changes_request::GetRecentAddressBalanceChangesRequestV0 {
                        start_height: self.start_height,
                        prove,
                        start_height_exclusive: false,
                    },
                ),
            ),
        })
    }
}

/// Query for fetching recent compacted address balance changes starting from a block height
#[derive(Debug, Clone)]
pub struct RecentCompactedAddressBalanceChangesQuery {
    /// The block height to start fetching from
    pub start_block_height: u64,
}

impl RecentCompactedAddressBalanceChangesQuery {
    /// Create a new query starting from a specific block height
    pub fn new(start_block_height: u64) -> Self {
        Self { start_block_height }
    }
}

impl Query<proto::GetRecentCompactedAddressBalanceChangesRequest>
    for RecentCompactedAddressBalanceChangesQuery
{
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<proto::GetRecentCompactedAddressBalanceChangesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(proto::GetRecentCompactedAddressBalanceChangesRequest {
            version: Some(
                proto::get_recent_compacted_address_balance_changes_request::Version::V0(
                    proto::get_recent_compacted_address_balance_changes_request::GetRecentCompactedAddressBalanceChangesRequestV0 {
                        start_block_height: self.start_block_height,
                        prove,
                    },
                ),
            ),
        })
    }
}

// --- Shielded Pool Queries ---

impl Query<GetShieldedPoolStateRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetShieldedPoolStateRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetShieldedPoolStateRequest {
            version: Some(get_shielded_pool_state_request::Version::V0(
                get_shielded_pool_state_request::GetShieldedPoolStateRequestV0 { prove },
            )),
        })
    }
}

impl Query<GetShieldedNotesCountRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetShieldedNotesCountRequest, Error> {
        let prove = settings.prove;
        // GetShieldedNotesCount is proved-only: the count is bound by the
        // Merk value hash, so it is always returned inside a verifiable
        // proof. Return a recoverable error (not a process-aborting
        // `unimplemented!`) if a caller explicitly disables proofs on this
        // public request type.
        if !prove {
            return Err(Error::Generic(
                "GetShieldedNotesCount requires proofs; unproved queries are not supported"
                    .to_string(),
            ));
        }

        Ok(GetShieldedNotesCountRequest {
            version: Some(get_shielded_notes_count_request::Version::V0(
                get_shielded_notes_count_request::GetShieldedNotesCountRequestV0 { prove },
            )),
        })
    }
}

impl Query<GetShieldedAnchorsRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetShieldedAnchorsRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetShieldedAnchorsRequest {
            version: Some(get_shielded_anchors_request::Version::V0(
                get_shielded_anchors_request::GetShieldedAnchorsRequestV0 { prove },
            )),
        })
    }
}

impl Query<GetMostRecentShieldedAnchorRequest> for NoParamQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetMostRecentShieldedAnchorRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetMostRecentShieldedAnchorRequest {
            version: Some(get_most_recent_shielded_anchor_request::Version::V0(
                get_most_recent_shielded_anchor_request::GetMostRecentShieldedAnchorRequestV0 {
                    prove,
                },
            )),
        })
    }
}

impl Query<GetShieldedEncryptedNotesRequest> for ShieldedEncryptedNotesQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetShieldedEncryptedNotesRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetShieldedEncryptedNotesRequest {
            version: Some(get_shielded_encrypted_notes_request::Version::V0(
                get_shielded_encrypted_notes_request::GetShieldedEncryptedNotesRequestV0 {
                    start_index: self.start_index,
                    count: self.count,
                    prove,
                },
            )),
        })
    }
}

impl Query<GetShieldedNullifiersRequest> for ShieldedNullifiersQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetShieldedNullifiersRequest, Error> {
        let prove = settings.prove;
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        Ok(GetShieldedNullifiersRequest {
            version: Some(get_shielded_nullifiers_request::Version::V0(
                get_shielded_nullifiers_request::GetShieldedNullifiersRequestV0 {
                    nullifiers: self.0.iter().map(|n| n.to_vec()).collect(),
                    prove,
                },
            )),
        })
    }
}
