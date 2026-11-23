//! Contract moderation queries: one identity's status on a moderated contract
//! (`getContractModerationStatus`), one page of a contract's banlist, suspension list or
//! warning list (`getContractModerationEntries`), the records of the documents its
//! moderators deleted (`getContractDocumentRemovals`), the actions its seated moderation team
//! votes on (`getContractTeamActions`) with who approved each (`getContractTeamActionSigners`),
//! and how many moderation actions each member of that team signed since the moderators pot was
//! last paid out (`getContractModerationActionCounts`).
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
//! * [`ContractTeamActions::fetch`] with a [`ContractTeamActionsPageQuery`] returns one page
//!   of the actions an elected contract's seated moderation team votes on, active (still
//!   gathering approvals) or closed (their approvals met the rule and they ran), in action id
//!   order; the page's [`after`](ContractTeamActionsPageQuery::after) is the query of the next
//!   page. Today the one action is the deletion of a settled document
//!   (`moderatorAbilities.deleteSettled`): a member proposes it, which is its own approval, and
//!   the others approve it by its id.
//! * [`ContractTeamActionSigners::fetch`] with a [`ContractTeamActionSignersQuery`] returns who
//!   approved one of those actions, the proposer among them unless it left the team and its
//!   approval was dropped, in identity id order: none when the contract holds no action of that
//!   id with the status asked.
//! * [`ContractModerationActionCounts::fetch`] with a [`ContractModerationActionCountsQuery`] (or
//!   the contract id alone) returns how many counted moderation actions (a ban, a suspension, a
//!   warning or a document deletion) each member of an elected contract's seated team signed
//!   since the moderators pot was last paid out, which resets every count: what a payout by
//!   actions shares that part of the pot by. A member that did not act since has no count. Only
//!   an elected contract keeps counts; the node refuses any other.
//!
//! Every type also implements [`FetchUnproved`] for the unverified fast path.

use crate::platform::documents::contest_vote_state::read_not_older_than;
use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{
    DocumentIds, GetContractDocumentRemovalsRequestV0, Page,
};
use dapi_grpc::platform::v0::get_contract_moderation_action_counts_request::GetContractModerationActionCountsRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_contract_team_action_signers_request::{
    ActionStatus as TeamActionSignersStatus, GetContractTeamActionSignersRequestV0,
};
use dapi_grpc::platform::v0::get_contract_team_actions_request::{
    ActionStatus as TeamActionsStatus, GetContractTeamActionsRequestV0, StartAtActionId,
};
use dapi_grpc::platform::v0::{
    get_contract_document_removals_request, get_contract_moderation_action_counts_request,
    get_contract_moderation_entries_request, get_contract_moderation_status_request,
    get_contract_team_action_signers_request, get_contract_team_actions_request,
    GetContractDocumentRemovalsRequest, GetContractModerationActionCountsRequest,
    GetContractModerationEntriesRequest, GetContractModerationStatusRequest,
    GetContractTeamActionSignersRequest, GetContractTeamActionsRequest,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
pub use drive_proof_verifier::types::contract_moderation::{
    default_contract_document_removals_limit, default_contract_moderation_entries_limit,
    list_to_request, ContractDocumentRemoval, ContractDocumentRemovalEntry,
    ContractDocumentRemovals, ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
    ContractModerationActionCounts, ContractModerationEntries, ContractModerationEntriesQuery, ContractModerationEntry,
    ContractModerationList, ContractModerationListStatus, ContractModerationListStatuses,
    ContractTeamAction, ContractTeamActionEntry, ContractTeamActionEvent,
    ContractTeamActionSigners, ContractTeamActions, ContractTeamActionsQuery, ContractWarning,
    GroupActionStatus,
};
use std::collections::BTreeMap;

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

    /// The status query of `identity_id` on the contract `contract_id`: over `lists` when the
    /// caller names them (an empty selection included, which the node judges), else over
    /// every list the contract keeps ([`for_contract_id`](Self::for_contract_id)).
    ///
    /// # Errors
    /// Those of [`for_contract_id`](Self::for_contract_id) when `lists` is `None`; none
    /// otherwise.
    pub async fn resolve(
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: Option<Vec<ContractModerationList>>,
    ) -> Result<Self, Error> {
        match lists {
            Some(lists) => Ok(Self {
                contract_id,
                identity_id,
                lists,
            }),
            None => Self::for_contract_id(sdk, contract_id, identity_id).await,
        }
    }

    /// The status of `identity_id` on every list the contract `contract_id` keeps, for a
    /// caller that holds only the contract's id: fetches the contract and reads its lists with
    /// [`for_contract`](Self::for_contract). Which lists a contract keeps is fixed when it is
    /// created, so the query stays valid for as long as the contract exists.
    ///
    /// The status request's own protocol requirement is checked first, through the gate the
    /// fetch paths use, so a network that does not serve the status read refuses it as the
    /// fetch would, before the contract is fetched.
    ///
    /// # Errors
    /// * [`Error::ProtocolVersionNotActive`] when the network is not proven to serve the
    ///   status request;
    /// * [`Error::MissingDependency`] when the network holds no such contract;
    /// * [`Error::InvalidRequest`] when the contract is not moderated, or is moderated and
    ///   keeps no list (its moderators only delete documents): no identity has a status on it,
    ///   and the node would refuse the query.
    pub async fn for_contract_id(
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
    ) -> Result<Self, Error> {
        // The requirement is the request's, whatever lists it names
        let unresolved = Self {
            contract_id,
            identity_id,
            lists: Vec::new(),
        };
        let requirement = Query::<GetContractModerationStatusRequest>::protocol_requirement(
            &unresolved,
            &sdk.query_settings(),
        )?;
        sdk.ensure_network_serves(requirement, None).await?;
        let contract = DataContract::fetch(sdk, contract_id)
            .await?
            .ok_or_else(|| {
                Error::MissingDependency("DataContract".to_string(), format!("{contract_id}"))
            })?;
        Self::for_contract(&contract, identity_id).ok_or_else(|| {
            let reason = match contract.config().moderation() {
                Some(_) => "keeps no moderation list, so no identity has a status on it",
                None => "is not moderated",
            };
            Error::InvalidRequest(format!("contract {contract_id} {reason}"))
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

impl Sdk {
    /// The status of `identity_id` on the contract `contract_id`: resolves the query
    /// ([`ContractModerationStatusQuery::resolve`]: over `lists` when named, an empty selection
    /// included, else over every list the contract keeps) and fetches it
    /// ([`ContractModerationListStatuses::fetch`]).
    ///
    /// # Errors
    /// Those of [`ContractModerationStatusQuery::resolve`] and of the fetch.
    pub async fn fetch_contract_moderation_status(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: Option<Vec<ContractModerationList>>,
    ) -> Result<Option<ContractModerationListStatuses>, Error> {
        let query =
            ContractModerationStatusQuery::resolve(self, contract_id, identity_id, lists).await?;
        ContractModerationListStatuses::fetch(self, query).await
    }
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

    /// The query for the page after `page`, or `None` when `page` holds fewer records than the
    /// limit and so was the last. A read by ids has no page after it.
    pub fn after(&self, page: &ContractDocumentRemovals) -> Option<Self> {
        page.next_query(&self.query).map(|query| Self {
            contract_id: self.contract_id,
            query,
        })
    }

    /// Reads the records this query names, each with the values it keeps of its document
    /// ([`ContractDocumentRemovals::decode_kept_values`], sharing
    /// `DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET` across the whole page), and the query of the
    /// page after them. The kept values are encoded under the document's type, so the contract is
    /// fetched when a record keeps any; a read whose records keep none costs the records' request
    /// alone. `None` when [`ContractDocumentRemovals::fetch`] returns none.
    ///
    /// A compatible contract update can add members to a kept object, and the values read
    /// only the members the type they are read under declares. So the contract must be
    /// answered at a block no older than the records: an older answer is asked again, three
    /// attempts in all ([`read_not_older_than`]). That budget, and the
    /// [`StaleNodeError::Height`](crate::error::StaleNodeError::Height) it ends in, apply only
    /// to contract answers that pass the contract fetch's own verification. An answer further
    /// behind than the SDK's height tolerance already fails inside that fetch, under the
    /// fetch's own retry policy, and surfaces as the fetch's error (an exhaustion wrapping a
    /// stale error, for example) instead.
    ///
    /// The values are read under the document type as the contract holds it now, as Drive and
    /// every other reader read them: a record stores the members of a kept object by position,
    /// not by name, and no layout of its own. So on a contract whose protocol 14 update
    /// reordered the members of a kept object (protocol 15 refuses such an update), a record
    /// written before the update reads its values under the new order's names.
    ///
    /// # Errors
    /// Those of [`ContractDocumentRemovals::fetch`], and, when a record keeps values:
    /// * [`Error::MissingDependency`] when the network holds no such contract;
    /// * [`Error::StaleNode`] when every contract answer that passed the fetch's verification
    ///   is from a block older than the records'
    ///   ([`StaleNodeError::Height`](crate::error::StaleNodeError::Height), `expected_height`
    ///   the records' block); an answer beyond the SDK's height tolerance fails inside the
    ///   fetch with that fetch's error;
    /// * [`Error::Protocol`] when the contract declares no such document type, or the record's
    ///   values do not read under it.
    pub async fn fetch_with_kept_values(
        &self,
        sdk: &Sdk,
    ) -> Result<Option<ContractDocumentRemovalsWithKeptValues>, Error> {
        let (page, removals_metadata) =
            ContractDocumentRemovals::fetch_with_metadata(sdk, self.clone(), None).await?;
        let Some(page) = page else {
            return Ok(None);
        };
        let next = self.after(&page);
        let keeps_values = page
            .removals()
            .iter()
            .any(|entry| !entry.removal.kept_fields.is_empty());
        let contract = if keeps_values {
            Some(
                self.contract_at_or_after(sdk, removals_metadata.height)
                    .await?,
            )
        } else {
            None
        };
        let document_type = contract
            .as_ref()
            .map(|contract| {
                contract
                    .document_type_for_name(&self.query.document_type_name)
                    .map_err(ProtocolError::from)
            })
            .transpose()?;
        // Share one decoded-value budget across every removal record in the page; a page
        // whose records keep nothing skips contract fetching and reads as empty maps.
        let kept_values_list = match document_type {
            Some(document_type) => page.decode_kept_values(document_type)?,
            None => vec![BTreeMap::new(); page.0.len()],
        };
        let removals = page.0.into_iter().zip(kept_values_list).collect();
        Ok(Some(ContractDocumentRemovalsWithKeptValues {
            removals,
            next,
        }))
    }

    /// The contract as answered at a block no older than `records_height`, asked again while
    /// an answer is older ([`read_not_older_than`]). An absence is proved at a height too, so
    /// it follows the same rule: only an absence no older than the records means the contract
    /// is gone. The attempts count only answers the fetch verified: one beyond the SDK's height
    /// tolerance fails inside the fetch, with its own retries and error.
    async fn contract_at_or_after(
        &self,
        sdk: &Sdk,
        records_height: u64,
    ) -> Result<DataContract, Error> {
        let (contract, _) = read_not_older_than(
            records_height,
            |_| false,
            || DataContract::fetch_with_metadata(sdk, self.contract_id, None),
        )
        .await?;
        contract.ok_or_else(|| {
            Error::MissingDependency("DataContract".to_string(), format!("{}", self.contract_id))
        })
    }
}

/// Removal records of one document type, each with the values it keeps read under that type:
/// what [`ContractDocumentRemovalsPageQuery::fetch_with_kept_values`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDocumentRemovalsWithKeptValues {
    /// The records, in document id order, each with the values it keeps by property path
    /// (`moderatorAbilities.deleteKeepsFields`): empty when it keeps none, and a path the
    /// document held no value at left out.
    pub removals: Vec<(ContractDocumentRemovalEntry, BTreeMap<String, Value>)>,
    /// The query of the next page; `None` when the page held fewer records than the limit and
    /// so was the last, and for a read by ids, which has no page after it.
    pub next: Option<ContractDocumentRemovalsPageQuery>,
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

/// The wire number of a team action status in a team actions request.
fn team_actions_status_to_request(status: GroupActionStatus) -> i32 {
    match status {
        GroupActionStatus::ActionActive => TeamActionsStatus::Active as i32,
        GroupActionStatus::ActionClosed => TeamActionsStatus::Closed as i32,
    }
}

/// The wire number of a team action status in a team action signers request, whose enum is its
/// own though it carries the same two statuses.
fn team_action_signers_status_to_request(status: GroupActionStatus) -> i32 {
    match status {
        GroupActionStatus::ActionActive => TeamActionSignersStatus::Active as i32,
        GroupActionStatus::ActionClosed => TeamActionSignersStatus::Closed as i32,
    }
}

/// Query for one page of the actions an elected contract's seated moderation team votes on:
/// the active ones, still gathering approvals, or the closed ones, whose approvals met their
/// rule and which ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTeamActionsPageQuery {
    /// The elected contract.
    pub contract_id: Identifier,
    /// The status, the cursor and the limit.
    pub query: ContractTeamActionsQuery,
}

impl ContractTeamActionsPageQuery {
    /// The first page of the actions of `status`, up to the page cap of `platform_version`: the
    /// version of the network queried (`Sdk::version`), whose cap is the one the node enforces.
    pub fn new(
        contract_id: Identifier,
        status: GroupActionStatus,
        platform_version: &PlatformVersion,
    ) -> Self {
        Self {
            contract_id,
            query: ContractTeamActionsQuery {
                status,
                start_at: None,
                // The same cap bounds every moderation page, team actions included.
                limit: default_contract_document_removals_limit(platform_version),
            },
        }
    }

    /// Starts the page at `action_id`, the action itself included when `included` is set.
    pub fn starting_at(mut self, action_id: Identifier, included: bool) -> Self {
        self.query.start_at = Some((action_id, included));
        self
    }

    /// Bounds the page to `limit` actions.
    pub fn with_limit(mut self, limit: u16) -> Self {
        self.query.limit = limit;
        self
    }

    /// The query for the page after `page`, or `None` when `page` holds fewer actions than the
    /// limit and so was the last.
    pub fn after(&self, page: &ContractTeamActions) -> Option<Self> {
        page.next_query(&self.query).map(|query| Self {
            contract_id: self.contract_id,
            query,
        })
    }
}

impl Query<GetContractTeamActionsRequest> for ContractTeamActionsPageQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetContractTeamActionsRequest, Error> {
        Ok(GetContractTeamActionsRequest {
            version: Some(get_contract_team_actions_request::Version::V0(
                GetContractTeamActionsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    status: team_actions_status_to_request(self.query.status),
                    start_at_action_id: self.query.start_at.map(|(action_id, included)| {
                        StartAtActionId {
                            start_action_id: action_id.to_vec(),
                            start_action_id_included: included,
                        }
                    }),
                    count: Some(u32::from(self.query.limit)),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractTeamActions {
    type Query = GetContractTeamActionsRequest;
    type Request = GetContractTeamActionsRequest;
}

impl FetchUnproved for ContractTeamActions {
    type Request = GetContractTeamActionsRequest;
}

/// Query for who approved one of the actions an elected contract's seated moderation team
/// votes on, the proposer among them unless it left the team and its approval was dropped. The
/// action is looked up among the actions of `status` alone: an action that closed is no longer
/// among the active ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractTeamActionSignersQuery {
    /// The elected contract.
    pub contract_id: Identifier,
    /// Whether the action is active or closed.
    pub status: GroupActionStatus,
    /// The action.
    pub action_id: Identifier,
}

impl Query<GetContractTeamActionSignersRequest> for ContractTeamActionSignersQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractTeamActionSignersRequest, Error> {
        Ok(GetContractTeamActionSignersRequest {
            version: Some(get_contract_team_action_signers_request::Version::V0(
                GetContractTeamActionSignersRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    status: team_action_signers_status_to_request(self.status),
                    action_id: self.action_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for ContractTeamActionSigners {
    type Query = GetContractTeamActionSignersRequest;
    type Request = GetContractTeamActionSignersRequest;
}

impl FetchUnproved for ContractTeamActionSigners {
    type Request = GetContractTeamActionSignersRequest;
}

/// Query for how many moderation actions each member of an elected contract's seated team
/// signed since the moderators pot was last paid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationActionCountsQuery {
    /// The elected contract.
    pub contract_id: Identifier,
}

impl Query<GetContractModerationActionCountsRequest> for ContractModerationActionCountsQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractModerationActionCountsRequest, Error> {
        Ok(GetContractModerationActionCountsRequest {
            version: Some(get_contract_moderation_action_counts_request::Version::V0(
                GetContractModerationActionCountsRequestV0 {
                    contract_id: self.contract_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Query<GetContractModerationActionCountsRequest> for Identifier {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetContractModerationActionCountsRequest, Error> {
        ContractModerationActionCountsQuery { contract_id: *self }.query(settings)
    }
}

impl Fetch for ContractModerationActionCounts {
    type Query = GetContractModerationActionCountsRequest;
    type Request = GetContractModerationActionCountsRequest;
}

impl FetchUnproved for ContractModerationActionCounts {
    type Request = GetContractModerationActionCountsRequest;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::moderation::ContractModerationReason;
    use drive_proof_verifier::types::contract_moderation::{
        team_action_status_from_request, team_actions_query_from_request,
    };
    use rs_dapi_client::RequestSettings;

    fn query_settings(request_settings: &RequestSettings) -> QuerySettings<'_> {
        QuerySettings {
            request_settings,
            protocol_version: PlatformVersion::latest(),
            verified_protocol_version: 0,
            prove: true,
        }
    }

    fn team_action_entry(seed: u8) -> ContractTeamActionEntry {
        ContractTeamActionEntry {
            action_id: Identifier::from([seed; 32]),
            action: ContractTeamAction {
                proposer_id: Identifier::from([seed.wrapping_add(1); 32]),
                proposed_at: 1_000,
                event: ContractTeamActionEvent::DeleteSettledDocument {
                    document_type_name: "post".to_string(),
                    document_id: Identifier::from([seed.wrapping_add(2); 32]),
                    document_last_modified_at: 10,
                    document_revision: Some(1),
                    reason: ContractModerationReason::from_text("spam"),
                },
            },
            approval_count: u32::from(seed),
        }
    }

    #[test]
    fn should_send_a_team_actions_page_the_verifier_reads_back_as_the_same_query() {
        let request_settings = RequestSettings::default();
        let settings = query_settings(&request_settings);
        let contract_id = Identifier::from([1; 32]);
        for status in [
            GroupActionStatus::ActionActive,
            GroupActionStatus::ActionClosed,
        ] {
            let page =
                ContractTeamActionsPageQuery::new(contract_id, status, PlatformVersion::latest())
                    .starting_at(Identifier::from([9; 32]), true)
                    .with_limit(5);
            let request = page.query(&settings).expect("request");
            let Some(get_contract_team_actions_request::Version::V0(v0)) = request.version else {
                panic!("expected a v0 request");
            };
            assert_eq!(v0.contract_id, contract_id.to_vec());
            assert!(v0.prove);
            assert_eq!(
                team_actions_query_from_request(
                    v0.status,
                    v0.start_at_action_id,
                    v0.count,
                    PlatformVersion::latest(),
                )
                .expect("query"),
                page.query
            );
        }
    }

    #[test]
    fn should_send_the_status_of_a_team_action_signers_query() {
        let request_settings = RequestSettings::default();
        let settings = query_settings(&request_settings);
        for status in [
            GroupActionStatus::ActionActive,
            GroupActionStatus::ActionClosed,
        ] {
            let query = ContractTeamActionSignersQuery {
                contract_id: Identifier::from([1; 32]),
                status,
                action_id: Identifier::from([2; 32]),
            };
            let request = query.query(&settings).expect("request");
            let Some(get_contract_team_action_signers_request::Version::V0(v0)) = request.version
            else {
                panic!("expected a v0 request");
            };
            assert_eq!(v0.contract_id, query.contract_id.to_vec());
            assert_eq!(v0.action_id, query.action_id.to_vec());
            assert_eq!(
                team_action_status_from_request(v0.status).expect("status"),
                status
            );
        }
    }

    #[test]
    fn should_ask_for_the_moderation_action_counts_of_a_contract() {
        let request_settings = RequestSettings::default();
        let settings = query_settings(&request_settings);
        let contract_id = Identifier::from([1; 32]);
        let request: GetContractModerationActionCountsRequest =
            contract_id.query(&settings).expect("request");
        assert_eq!(
            request,
            ContractModerationActionCountsQuery { contract_id }
                .query(&settings)
                .expect("request")
        );
        let Some(get_contract_moderation_action_counts_request::Version::V0(v0)) = request.version
        else {
            panic!("expected a v0 request");
        };
        assert_eq!(v0.contract_id, contract_id.to_vec());
        assert!(v0.prove);
    }

    #[test]
    fn should_continue_team_actions_after_a_full_page_only() {
        let page_query = ContractTeamActionsPageQuery::new(
            Identifier::from([1; 32]),
            GroupActionStatus::ActionActive,
            PlatformVersion::latest(),
        )
        .with_limit(2);
        let full = ContractTeamActions(vec![team_action_entry(3), team_action_entry(4)]);
        let next = page_query.after(&full).expect("a page follows a full one");
        assert_eq!(next.contract_id, page_query.contract_id);
        assert_eq!(
            next.query,
            ContractTeamActionsQuery {
                status: GroupActionStatus::ActionActive,
                start_at: Some((Identifier::from([4; 32]), false)),
                limit: 2,
            }
        );
        let short = ContractTeamActions(vec![team_action_entry(3)]);
        assert_eq!(page_query.after(&short), None);
    }
}
