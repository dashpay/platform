//! Contract moderation queries: one identity's status on a moderated contract
//! (`getContractModerationStatus`), one page of a contract's banlist, suspension list or
//! warning list (`getContractModerationEntries`), the records of the documents its
//! moderators deleted (`getContractDocumentRemovals`) and the approvals its seated moderation
//! team gave the deletion of settled documents (`getContractSettledDeletions`).
//!
//! A moderated contract declares in its config which lists it keeps
//! (`DataContractConfig::moderation`). A status query names the lists to read, and each must be
//! one the contract keeps: a list the contract does not keep has no tree and the node refuses
//! the query. [`ContractModerationStatusQuery::for_contract`] derives the lists from a contract
//! the caller holds.
//!
//! * [`ContractModerationListStatuses::fetch`] with a [`ContractModerationStatusQuery`] returns
//!   the identity's status on each list queried; a list not queried is absent, not empty.
//! * [`ContractModerationEntries::fetch`] with a [`ContractModerationEntriesPageQuery`] returns
//!   one page of a list in identity id order; the page's
//!   [`next_query`](ContractModerationEntries::next_query) is the cursor of the next page.
//! * [`ContractDocumentRemovals::fetch`] with a [`ContractDocumentRemovalsPageQuery`] returns
//!   the records of the documents the moderators deleted within one document type, either one
//!   page in document id order or the records of the ids named. The document type must be one
//!   whose moderators' deletions keep records (`moderatorAbilities.delete`, without
//!   `deleteKeepsRecord: false`): no other keeps records, and the node refuses a query over a
//!   tree that does not exist.
//! * [`ContractSettledDeletions::fetch`] with a [`ContractSettledDeletionsPageQuery`] returns
//!   the approvals a seated moderation team gave the deletion of settled documents within one
//!   document type, selected as removal records are: one record per document, open, lapsed, or
//!   met and the document deleted (`deleted_at`). The document type must say who of the team
//!   approves such a deletion (`moderatorAbilities.deleteSettled`): no other keeps approvals,
//!   and the node refuses the query.
//!
//! Every type also implements [`FetchUnproved`] for the unverified fast path.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{
    DocumentIds, GetContractDocumentRemovalsRequestV0, Page,
};
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_contract_settled_deletions_request::get_contract_settled_deletions_request_v0::Selection as SettledDeletionsSelection;
use dapi_grpc::platform::v0::get_contract_settled_deletions_request::{
    DocumentIds as SettledDeletionsDocumentIds, GetContractSettledDeletionsRequestV0,
    Page as SettledDeletionsPage,
};
use dapi_grpc::platform::v0::{
    get_contract_document_removals_request, get_contract_moderation_entries_request,
    get_contract_moderation_status_request, get_contract_settled_deletions_request,
    GetContractDocumentRemovalsRequest, GetContractModerationEntriesRequest,
    GetContractModerationStatusRequest, GetContractSettledDeletionsRequest,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
pub use drive_proof_verifier::types::contract_moderation::{
    default_contract_document_removals_limit, default_contract_moderation_entries_limit,
    list_to_request, ContractDocumentRemoval,
    ContractDocumentRemovalEntry, ContractDocumentRemovals, ContractDocumentRemovalsQuery,
    ContractDocumentRemovalsSelection, ContractModerationEntries, ContractModerationEntriesQuery,
    ContractModerationEntry, ContractModerationList, ContractModerationListStatus,
    ContractModerationListStatuses, ContractSettledDeletion, ContractSettledDeletionEntry,
    ContractSettledDeletions, ContractSettledDeletionsQuery, ContractWarning,
};

/// Query for one identity's status on a moderated contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationStatusQuery {
    /// The moderated contract.
    pub contract_id: Identifier,
    /// The identity.
    pub identity_id: Identifier,
    /// The lists to read; each must be one the contract keeps.
    pub lists: Vec<ContractModerationList>,
}

impl ContractModerationStatusQuery {
    /// The status of `identity_id` on every list `contract` keeps, or `None` when the contract
    /// keeps no list (it is not moderated, so nobody is barred on it).
    pub fn for_contract(contract: &DataContract, identity_id: Identifier) -> Option<Self> {
        let lists: Vec<ContractModerationList> = contract.config().moderation()?.lists().collect();
        (!lists.is_empty()).then_some(Self {
            contract_id: contract.id(),
            identity_id,
            lists,
        })
    }
}

impl Query<GetContractModerationStatusRequest> for ContractModerationStatusQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractModerationStatusRequest, Error> {
        Ok(GetContractModerationStatusRequest {
            version: Some(get_contract_moderation_status_request::Version::V0(
                GetContractModerationStatusRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    identity_id: self.identity_id.to_vec(),
                    lists: self
                        .lists
                        .iter()
                        .map(|list| list_to_request(*list))
                        .collect(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

/// Query for one page of one moderation list of a contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationEntriesPageQuery {
    /// The moderated contract.
    pub contract_id: Identifier,
    /// The list, the cursor and the limit.
    pub query: ContractModerationEntriesQuery,
}

impl ContractModerationEntriesPageQuery {
    /// The first page of `list`, up to the page cap of `platform_version`: the version of the
    /// network queried (`Sdk::version`), whose cap is the one the node enforces.
    pub fn new(
        contract_id: Identifier,
        list: ContractModerationList,
        platform_version: &PlatformVersion,
    ) -> Self {
        Self {
            contract_id,
            query: ContractModerationEntriesQuery {
                list,
                start_after: None,
                limit: default_contract_moderation_entries_limit(platform_version),
            },
        }
    }

    /// Bounds the page to `limit` entries.
    pub fn with_limit(mut self, limit: u16) -> Self {
        self.query.limit = limit;
        self
    }

    /// The query for the page after `page`, or `None` when `page` holds fewer entries than the
    /// limit and so was the last.
    pub fn after(&self, page: &ContractModerationEntries) -> Option<Self> {
        page.next_query(&self.query).map(|query| Self {
            contract_id: self.contract_id,
            query,
        })
    }
}

impl Query<GetContractModerationEntriesRequest> for ContractModerationEntriesPageQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractModerationEntriesRequest, Error> {
        Ok(GetContractModerationEntriesRequest {
            version: Some(get_contract_moderation_entries_request::Version::V0(
                GetContractModerationEntriesRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    list: list_to_request(self.query.list),
                    start_after: self.query.start_after.map(|id| id.to_vec()),
                    limit: Some(u32::from(self.query.limit)),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractModerationListStatuses {
    type Query = GetContractModerationStatusRequest;
    type Request = GetContractModerationStatusRequest;
}

impl FetchUnproved for ContractModerationListStatuses {
    type Request = GetContractModerationStatusRequest;
}

impl Fetch for ContractModerationEntries {
    type Query = GetContractModerationEntriesRequest;
    type Request = GetContractModerationEntriesRequest;
}

impl FetchUnproved for ContractModerationEntries {
    type Request = GetContractModerationEntriesRequest;
}

/// Query for the removal records a moderated contract keeps for one of its document types:
/// one page of them, or the records of the document ids named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractDocumentRemovalsPageQuery {
    /// The moderated contract.
    pub contract_id: Identifier,
    /// The document type, and which of its records to read.
    pub query: ContractDocumentRemovalsQuery,
}

impl ContractDocumentRemovalsPageQuery {
    /// The first page of the records of `document_type_name`, up to the page cap of
    /// `platform_version`: the version of the network queried (`Sdk::version`), whose cap is
    /// the one the node enforces.
    pub fn new(
        contract_id: Identifier,
        document_type_name: String,
        platform_version: &PlatformVersion,
    ) -> Self {
        Self {
            contract_id,
            query: ContractDocumentRemovalsQuery {
                document_type_name,
                selection: ContractDocumentRemovalsSelection::Page {
                    start_after: None,
                    limit: default_contract_document_removals_limit(platform_version),
                },
            },
        }
    }

    /// The records of `document_ids` alone, from one to the page cap of the network queried.
    /// A document with no record is left out of the answer rather than refused.
    pub fn for_document_ids(
        contract_id: Identifier,
        document_type_name: String,
        document_ids: Vec<Identifier>,
    ) -> Self {
        Self {
            contract_id,
            query: ContractDocumentRemovalsQuery {
                document_type_name,
                selection: ContractDocumentRemovalsSelection::DocumentIds(document_ids),
            },
        }
    }

    /// Bounds the page to `limit` records, leaving a read by ids as it is: it is already
    /// bounded by the ids it names.
    pub fn with_limit(mut self, limit: u16) -> Self {
        if let ContractDocumentRemovalsSelection::Page { start_after, .. } = &self.query.selection {
            self.query.selection = ContractDocumentRemovalsSelection::Page {
                start_after: *start_after,
                limit,
            };
        }
        self
    }

    /// The query for the page after `page`, removal records or settled deletion approvals, or
    /// `None` when `page` holds fewer records than the limit and so was the last. A read by ids
    /// has no page after it.
    pub fn after<P: ContractDocumentRecordsPage>(&self, page: &P) -> Option<Self> {
        page.next_query(&self.query).map(|query| Self {
            contract_id: self.contract_id,
            query,
        })
    }
}

/// A page of the records a moderated contract keeps by document id, within one document type:
/// removal records, or the approvals of settled deletions. Both are read with one query.
pub trait ContractDocumentRecordsPage {
    /// The query for the page after this one, `None` when this one was the last.
    fn next_query(
        &self,
        query: &ContractDocumentRemovalsQuery,
    ) -> Option<ContractDocumentRemovalsQuery>;
}

impl ContractDocumentRecordsPage for ContractDocumentRemovals {
    fn next_query(
        &self,
        query: &ContractDocumentRemovalsQuery,
    ) -> Option<ContractDocumentRemovalsQuery> {
        ContractDocumentRemovals::next_query(self, query)
    }
}

impl ContractDocumentRecordsPage for ContractSettledDeletions {
    fn next_query(
        &self,
        query: &ContractDocumentRemovalsQuery,
    ) -> Option<ContractDocumentRemovalsQuery> {
        ContractSettledDeletions::next_query(self, query)
    }
}

impl Query<GetContractDocumentRemovalsRequest> for ContractDocumentRemovalsPageQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractDocumentRemovalsRequest, Error> {
        let selection = match &self.query.selection {
            ContractDocumentRemovalsSelection::DocumentIds(document_ids) => {
                Selection::DocumentIds(DocumentIds {
                    document_ids: document_ids.iter().map(|id| id.to_vec()).collect(),
                })
            }
            ContractDocumentRemovalsSelection::Page { start_after, limit } => {
                Selection::Page(Page {
                    start_after: start_after.map(|id| id.to_vec()),
                    limit: Some(u32::from(*limit)),
                })
            }
        };
        Ok(GetContractDocumentRemovalsRequest {
            version: Some(get_contract_document_removals_request::Version::V0(
                GetContractDocumentRemovalsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    document_type_name: self.query.document_type_name.clone(),
                    selection: Some(selection),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractDocumentRemovals {
    type Query = GetContractDocumentRemovalsRequest;
    type Request = GetContractDocumentRemovalsRequest;
}

impl FetchUnproved for ContractDocumentRemovals {
    type Request = GetContractDocumentRemovalsRequest;
}

/// Query for the approvals a moderated contract keeps of the deletion of settled documents of
/// one of its document types: one page of them, or the records of the document ids named. The
/// documents are selected as the removal records of one type are, so it is the same query, sent
/// as a settled deletions request when [`ContractSettledDeletions`] are fetched.
pub type ContractSettledDeletionsPageQuery = ContractDocumentRemovalsPageQuery;

impl Query<GetContractSettledDeletionsRequest> for ContractDocumentRemovalsPageQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractSettledDeletionsRequest, Error> {
        let selection = match &self.query.selection {
            ContractDocumentRemovalsSelection::DocumentIds(document_ids) => {
                SettledDeletionsSelection::DocumentIds(SettledDeletionsDocumentIds {
                    document_ids: document_ids.iter().map(|id| id.to_vec()).collect(),
                })
            }
            ContractDocumentRemovalsSelection::Page { start_after, limit } => {
                SettledDeletionsSelection::Page(SettledDeletionsPage {
                    start_after: start_after.map(|id| id.to_vec()),
                    limit: Some(u32::from(*limit)),
                })
            }
        };
        Ok(GetContractSettledDeletionsRequest {
            version: Some(get_contract_settled_deletions_request::Version::V0(
                GetContractSettledDeletionsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    document_type_name: self.query.document_type_name.clone(),
                    selection: Some(selection),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractSettledDeletions {
    type Query = GetContractSettledDeletionsRequest;
    type Request = GetContractSettledDeletionsRequest;
}

impl FetchUnproved for ContractSettledDeletions {
    type Request = GetContractSettledDeletionsRequest;
}
