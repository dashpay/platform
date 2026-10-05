//! Filters validated and bound to their data contracts, and matching.

use super::participants::{document_recipient, participants, token_id, token_recipient};
use super::{
    DocumentAction, DocumentActionMatch, DocumentFilter, Role, StateTransitionFilter,
    SubscriptionFilterError, MAX_ACTIONS_PER_DOCUMENT_FILTER, MAX_ADDRESSES_PER_FILTER,
    MAX_CLAUSES_PER_ACTION, MAX_FILTERS, MAX_IDS_PER_FILTER,
};
use dpp::address_funds::PlatformAddress;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::query::filter::{
    canonicalize_where_clause, DocumentActionMatchClauses, DriveDocumentQueryFilter,
    TransitionCheckResult,
};
use drive::query::{InternalClauses, ValueClause, WhereClause, WhereOperator};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Why a transition matched: the request filters it matched and, for a batch, the positions of
/// its document and token transitions that did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterMatch {
    /// Indexes of the matched filters, in request order.
    pub matched_filters: Vec<u32>,
    /// Positions, in the batch's transitions, of the inner transitions that matched.
    pub matched_batch_positions: Vec<u32>,
}

/// A subscription's filters, validated and bound to the data contracts their document filters
/// name.
#[derive(Debug, Clone)]
pub struct ResolvedFilters {
    filters: Vec<ResolvedFilter>,
}

#[derive(Debug, Clone)]
enum ResolvedFilter {
    Documents(ResolvedDocumentFilter),
    Addresses {
        addresses: BTreeSet<PlatformAddress>,
        role: Role,
    },
    Identities {
        identity_ids: BTreeSet<Identifier>,
        role: Role,
    },
    Tokens {
        token_ids: BTreeSet<Identifier>,
        identity_ids: BTreeSet<Identifier>,
        role: Role,
    },
    DataContracts {
        data_contract_ids: BTreeSet<Identifier>,
    },
}

#[derive(Debug, Clone)]
struct ResolvedDocumentFilter {
    /// The filter as requested, kept to rebind it to a newer version of its contract.
    source: DocumentFilter,
    contract: Arc<DataContract>,
    actions: ResolvedActions,
}

#[derive(Debug, Clone)]
enum ResolvedActions {
    /// Every action.
    Any,
    /// These actions, on any document type of the contract.
    Kinds(BTreeSet<DocumentAction>),
    /// One clause set per action alternative, on the filter's document type.
    Clauses(Vec<DocumentActionMatchClauses>),
}

impl ResolvedFilters {
    /// The data contracts the document filters among `filters` name; resolving needs them.
    pub fn data_contract_ids(filters: &[StateTransitionFilter]) -> BTreeSet<Identifier> {
        filters
            .iter()
            .filter_map(StateTransitionFilter::document_data_contract_id)
            .collect()
    }

    /// Validate `filters` and bind each document filter to its contract, looked up by
    /// `data_contract`. Clause values are brought to their fields' types under
    /// `platform_version`.
    ///
    /// The binding is to the contract as given, normally its current version, and changes
    /// only through [`Self::follow`]. Transitions replayed from before the latest update are
    /// therefore matched against a newer schema: contract updates are backwards compatible, so
    /// existing fields compare the same, but a `generatedFrom` property added since is generated
    /// for them too, so a clause on it can match a transition stored without it.
    pub fn resolve(
        filters: Vec<StateTransitionFilter>,
        data_contract: impl Fn(&Identifier) -> Option<Arc<DataContract>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, SubscriptionFilterError> {
        if filters.is_empty() {
            return Err(SubscriptionFilterError::InvalidRequest(
                "at least one filter is required".to_string(),
            ));
        }
        if filters.len() > MAX_FILTERS {
            return Err(SubscriptionFilterError::InvalidRequest(format!(
                "at most {MAX_FILTERS} filters are allowed, got {}",
                filters.len()
            )));
        }
        let filters = filters
            .into_iter()
            .enumerate()
            .map(|(index, filter)| resolve_filter(index, filter, &data_contract, platform_version))
            .collect::<Result<_, _>>()?;
        Ok(Self { filters })
    }

    /// Which filters `state_transition` matches, or `None` when it matches none.
    /// `platform_version` is the version of the block the transition executed in.
    pub fn matches(
        &self,
        state_transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> Option<FilterMatch> {
        let participants = participants(state_transition);
        let batch = match state_transition {
            StateTransition::Batch(batch) => Some(batch),
            _ => None,
        };
        let batch_owner_id = state_transition.owner_id().unwrap_or_default();
        let mut matched_filters = Vec::new();
        let mut matched_batch_positions = BTreeSet::new();

        for (index, filter) in self.filters.iter().enumerate() {
            let mut matched = match filter {
                ResolvedFilter::Addresses { addresses, role } => participants
                    .addresses
                    .iter()
                    .any(|(address, side)| role.admits(*side) && addresses.contains(address)),
                ResolvedFilter::Identities { identity_ids, role } => {
                    participants.identities.iter().any(|(identity_id, side)| {
                        role.admits(*side) && identity_ids.contains(identity_id)
                    })
                }
                ResolvedFilter::DataContracts { data_contract_ids } => participants
                    .data_contract_id
                    .is_some_and(|id| data_contract_ids.contains(&id)),
                // A shielded token transition outside a batch.
                ResolvedFilter::Tokens {
                    token_ids,
                    identity_ids,
                    role,
                } => participants.token_id.is_some_and(|token_id| {
                    (token_ids.is_empty() || token_ids.contains(&token_id))
                        && (identity_ids.is_empty()
                            || participants.identities.iter().any(|(identity_id, side)| {
                                role.admits(*side) && identity_ids.contains(identity_id)
                            }))
                }),
                ResolvedFilter::Documents(_) => false,
            };
            if let Some(batch) = batch {
                for (position, transition) in batch.transitions_iter().enumerate() {
                    if filter.matches_batched(transition, batch_owner_id, platform_version) {
                        matched = true;
                        matched_batch_positions.insert(position as u32);
                    }
                }
            }
            if matched {
                matched_filters.push(index as u32);
            }
        }

        (!matched_filters.is_empty()).then(|| FilterMatch {
            matched_filters,
            matched_batch_positions: matched_batch_positions.into_iter().collect(),
        })
    }

    /// Take note of a transition the stream has passed, matched or not: after a data contract
    /// update, the document filters on that contract match against the version it carries.
    ///
    /// This trusts the update's embedded contract, so only a party that trusts the block
    /// source may call it: the node, reading its own Tenderdash. A client must not install a
    /// contract a node sent it; it re-reads the contract with a proof instead (see
    /// [`Self::followed_data_contract`]). Fails when the update's contract cannot be built.
    pub fn follow(
        &mut self,
        state_transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let StateTransition::DataContractUpdate(update) = state_transition else {
            return Ok(());
        };
        if !self.binds(update.data_contract().id()) {
            return Ok(());
        }
        let contract = DataContract::try_from_platform_versioned(
            update.data_contract().clone(),
            false,
            &mut vec![],
            platform_version,
        )?;
        self.rebind_data_contract(Arc::new(contract), platform_version);
        Ok(())
    }

    /// The bound data contract a transition updates, if any: after it, the filters on that
    /// contract should be rebound to the contract's new version.
    pub fn followed_data_contract(&self, state_transition: &StateTransition) -> Option<Identifier> {
        match state_transition {
            StateTransition::DataContractUpdate(update)
                if self.binds(update.data_contract().id()) =>
            {
                Some(update.data_contract().id())
            }
            _ => None,
        }
    }

    /// Whether a document filter is bound to `data_contract_id`.
    fn binds(&self, data_contract_id: Identifier) -> bool {
        self.filters.iter().any(|filter| {
            matches!(filter, ResolvedFilter::Documents(document_filter)
                if document_filter.contract.id() == data_contract_id)
        })
    }

    /// The version of `data_contract_id` the document filters on it are bound to.
    pub fn data_contract(&self, data_contract_id: Identifier) -> Option<&DataContract> {
        self.filters.iter().find_map(|filter| match filter {
            ResolvedFilter::Documents(document_filter)
                if document_filter.contract.id() == data_contract_id =>
            {
                Some(document_filter.contract.as_ref())
            }
            _ => None,
        })
    }

    /// Rebind the document filters on `data_contract` to this version of it, as carried by a
    /// data contract update the stream has just passed. A filter the new version cannot serve
    /// (an older contract lacking its document type or a clause's field) keeps its binding.
    pub fn rebind_data_contract(
        &mut self,
        data_contract: Arc<DataContract>,
        platform_version: &PlatformVersion,
    ) {
        for filter in &mut self.filters {
            let ResolvedFilter::Documents(document_filter) = filter else {
                continue;
            };
            if document_filter.contract.id() != data_contract.id() {
                continue;
            }
            if let Ok(rebound) = resolve_document_filter(
                &document_filter.source,
                data_contract.clone(),
                platform_version,
            ) {
                *document_filter = rebound;
            }
        }
    }
}

impl ResolvedFilter {
    /// Whether one inner transition of a batch owned by `batch_owner_id` matches.
    fn matches_batched(
        &self,
        transition: BatchedTransitionRef,
        batch_owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> bool {
        match (self, transition) {
            (ResolvedFilter::Documents(filter), BatchedTransitionRef::Document(transition)) => {
                filter.matches(transition, batch_owner_id, platform_version)
            }
            (
                ResolvedFilter::Identities { identity_ids, role },
                BatchedTransitionRef::Document(transition),
            ) => {
                role.admits(Role::Recipient)
                    && document_recipient(transition).is_some_and(|id| identity_ids.contains(&id))
            }
            (
                ResolvedFilter::Identities { identity_ids, role },
                BatchedTransitionRef::Token(transition),
            ) => {
                role.admits(Role::Recipient)
                    && token_recipient(transition, batch_owner_id)
                        .is_some_and(|id| identity_ids.contains(&id))
            }
            (
                ResolvedFilter::Tokens {
                    token_ids,
                    identity_ids,
                    role,
                },
                BatchedTransitionRef::Token(transition),
            ) => {
                if !token_ids.is_empty() && !token_ids.contains(&token_id(transition)) {
                    return false;
                }
                if identity_ids.is_empty() {
                    return true;
                }
                (role.admits(Role::Sender) && identity_ids.contains(&batch_owner_id))
                    || (role.admits(Role::Recipient)
                        && token_recipient(transition, batch_owner_id)
                            .is_some_and(|id| identity_ids.contains(&id)))
            }
            _ => false,
        }
    }
}

impl ResolvedDocumentFilter {
    fn matches(
        &self,
        transition: &DocumentTransition,
        batch_owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> bool {
        let base = transition.base();
        if base.data_contract_id() != self.contract.id() {
            return false;
        }
        if let Some(document_type_name) = &self.source.document_type_name {
            if base.document_type_name() != document_type_name {
                return false;
            }
        }
        if self
            .source
            .batch_owner_id
            .is_some_and(|owner| owner != batch_owner_id)
        {
            return false;
        }
        match &self.actions {
            ResolvedActions::Any => true,
            ResolvedActions::Kinds(actions) => actions.contains(&action_of(transition)),
            ResolvedActions::Clauses(alternatives) => {
                let document_type_name = self.source.document_type_name.clone().unwrap_or_default();
                let batch_owner_value = Value::from(batch_owner_id);
                alternatives.iter().any(|action_clauses| {
                    let filter = DriveDocumentQueryFilter {
                        contract: &self.contract,
                        document_type_name: document_type_name.clone(),
                        action_clauses: action_clauses.clone(),
                    };
                    filter.matches_document_transition(
                        transition,
                        Some(&batch_owner_value),
                        platform_version,
                    ) == TransitionCheckResult::Pass
                })
            }
        }
    }
}

fn action_of(transition: &DocumentTransition) -> DocumentAction {
    match transition {
        DocumentTransition::Create(_) => DocumentAction::Create,
        DocumentTransition::Replace(_) => DocumentAction::Replace,
        DocumentTransition::Delete(_) | DocumentTransition::IndexOnlyDelete(_) => {
            DocumentAction::Delete
        }
        DocumentTransition::Transfer(_) => DocumentAction::Transfer,
        DocumentTransition::UpdatePrice(_) => DocumentAction::UpdatePrice,
        DocumentTransition::Purchase(_) => DocumentAction::Purchase,
    }
}

fn resolve_filter(
    index: usize,
    filter: StateTransitionFilter,
    data_contract: &impl Fn(&Identifier) -> Option<Arc<DataContract>>,
    platform_version: &PlatformVersion,
) -> Result<ResolvedFilter, SubscriptionFilterError> {
    let invalid = |message: String| SubscriptionFilterError::filter(index, message);
    let id_set = |ids: Vec<Identifier>, what: &str, allow_empty: bool| {
        if ids.len() > MAX_IDS_PER_FILTER {
            return Err(invalid(format!(
                "at most {MAX_IDS_PER_FILTER} {what} are allowed, got {}",
                ids.len()
            )));
        }
        if ids.is_empty() && !allow_empty {
            return Err(invalid(format!("at least one of {what} is required")));
        }
        Ok(ids.into_iter().collect::<BTreeSet<_>>())
    };
    Ok(match filter {
        StateTransitionFilter::Documents(filter) => {
            let contract = data_contract(&filter.data_contract_id).ok_or(
                SubscriptionFilterError::DataContractNotFound(filter.data_contract_id),
            )?;
            ResolvedFilter::Documents(
                resolve_document_filter(&filter, contract, platform_version).map_err(invalid)?,
            )
        }
        StateTransitionFilter::Addresses { addresses, role } => {
            if addresses.is_empty() || addresses.len() > MAX_ADDRESSES_PER_FILTER {
                return Err(invalid(format!(
                    "an address filter takes 1 to {MAX_ADDRESSES_PER_FILTER} addresses, got {}",
                    addresses.len()
                )));
            }
            ResolvedFilter::Addresses {
                addresses: addresses.into_iter().collect(),
                role,
            }
        }
        StateTransitionFilter::Identities { identity_ids, role } => ResolvedFilter::Identities {
            identity_ids: id_set(identity_ids, "identity ids", false)?,
            role,
        },
        StateTransitionFilter::Tokens {
            token_ids,
            identity_ids,
            role,
        } => {
            if token_ids.is_empty() && identity_ids.is_empty() {
                return Err(invalid(
                    "a token filter needs token ids, identity ids or both".to_string(),
                ));
            }
            ResolvedFilter::Tokens {
                token_ids: id_set(token_ids, "token ids", true)?,
                identity_ids: id_set(identity_ids, "identity ids", true)?,
                role,
            }
        }
        StateTransitionFilter::DataContracts { data_contract_ids } => {
            ResolvedFilter::DataContracts {
                data_contract_ids: id_set(data_contract_ids, "data contract ids", false)?,
            }
        }
    })
}

fn resolve_document_filter(
    filter: &DocumentFilter,
    contract: Arc<DataContract>,
    platform_version: &PlatformVersion,
) -> Result<ResolvedDocumentFilter, String> {
    if filter.actions.len() > MAX_ACTIONS_PER_DOCUMENT_FILTER {
        return Err(format!(
            "at most {MAX_ACTIONS_PER_DOCUMENT_FILTER} actions are allowed, got {}",
            filter.actions.len()
        ));
    }
    let actions = match &filter.document_type_name {
        None => {
            if filter
                .actions
                .iter()
                .any(DocumentActionMatch::has_constraints)
            {
                return Err("action clauses need a document type".to_string());
            }
            if filter.actions.is_empty() {
                ResolvedActions::Any
            } else {
                ResolvedActions::Kinds(
                    filter
                        .actions
                        .iter()
                        .map(|action| action.action.ok_or_else(missing_action))
                        .collect::<Result<_, _>>()?,
                )
            }
        }
        Some(document_type_name) => {
            if contract
                .document_type_optional_for_name(document_type_name)
                .is_none()
            {
                return Err(format!(
                    "data contract {} has no document type '{document_type_name}'",
                    contract.id()
                ));
            }
            if filter.actions.is_empty() {
                ResolvedActions::Any
            } else {
                ResolvedActions::Clauses(
                    filter
                        .actions
                        .iter()
                        .map(|action| {
                            action_clauses(action, &contract, document_type_name, platform_version)
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
        }
    };
    Ok(ResolvedDocumentFilter {
        source: filter.clone(),
        contract,
        actions,
    })
}

fn missing_action() -> String {
    "every action match must name its action".to_string()
}

/// The drive filter clauses for one action alternative, canonicalized and validated.
fn action_clauses(
    action: &DocumentActionMatch,
    contract: &DataContract,
    document_type_name: &str,
    platform_version: &PlatformVersion,
) -> Result<DocumentActionMatchClauses, String> {
    let kind = action.action.ok_or_else(missing_action)?;
    for (clauses, side) in [
        (&action.new_document_where, "new document"),
        (&action.original_document_where, "original document"),
    ] {
        if clauses.len() > MAX_CLAUSES_PER_ACTION {
            return Err(format!(
                "at most {MAX_CLAUSES_PER_ACTION} {side} clauses are allowed, got {}",
                clauses.len()
            ));
        }
    }
    if action.owner_ids.len() > MAX_IDS_PER_FILTER {
        return Err(format!(
            "at most {MAX_IDS_PER_FILTER} owner ids are allowed, got {}",
            action.owner_ids.len()
        ));
    }
    if !matches!(kind, DocumentAction::Create | DocumentAction::Replace)
        && !action.new_document_where.is_empty()
    {
        return Err("new document clauses apply only to create and replace".to_string());
    }
    if kind == DocumentAction::Create && !action.original_document_where.is_empty() {
        return Err("a create has no original document".to_string());
    }
    if !matches!(kind, DocumentAction::Transfer | DocumentAction::Purchase)
        && !action.owner_ids.is_empty()
    {
        return Err("owner ids apply only to transfer and purchase".to_string());
    }
    if kind != DocumentAction::UpdatePrice && action.price.is_some() {
        return Err("a price clause applies only to update price".to_string());
    }

    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| e.to_string())?;
    // Canonicalize before extraction groups range bounds, so bounds given with different
    // integer widths (`>= I64(1)`, `<= U64(5)` on a u8 field) group.
    let extract = |clauses: &Vec<WhereClause>| {
        let mut clauses = clauses.clone();
        for clause in &mut clauses {
            canonicalize_where_clause(document_type, clause, platform_version)
                .map_err(|e| e.to_string())?;
        }
        InternalClauses::extract_from_clauses(clauses, platform_version).map_err(|e| e.to_string())
    };
    let new_document_clauses = extract(&action.new_document_where)?;
    let original_document_clauses = extract(&action.original_document_where)?;

    // Of a document before the transition, a transition carries only its `$id` — except an
    // indexOnly document's delete, which carries its values.
    let index_only_delete = kind == DocumentAction::Delete
        && contract
            .document_type_optional_for_name(document_type_name)
            .is_some_and(|document_type| document_type.index_only());
    let primary_key_only = original_document_clauses.in_clauses.is_empty()
        && original_document_clauses.range_clause.is_none()
        && original_document_clauses.equal_clauses.is_empty();
    if !index_only_delete && !primary_key_only {
        return Err(
            "clauses on the document before the transition may only name `$id`".to_string(),
        );
    }

    let owner_clause = (!action.owner_ids.is_empty()).then(|| ValueClause {
        operator: WhereOperator::In,
        value: Value::Array(action.owner_ids.iter().copied().map(Value::from).collect()),
    });
    let action_clauses = match kind {
        DocumentAction::Create => DocumentActionMatchClauses::Create {
            new_document_clauses,
        },
        DocumentAction::Replace => DocumentActionMatchClauses::Replace {
            original_document_clauses,
            new_document_clauses,
        },
        DocumentAction::Delete => DocumentActionMatchClauses::Delete {
            original_document_clauses,
        },
        DocumentAction::Transfer => DocumentActionMatchClauses::Transfer {
            original_document_clauses,
            owner_clause,
        },
        DocumentAction::UpdatePrice => DocumentActionMatchClauses::UpdatePrice {
            original_document_clauses,
            price_clause: action.price.clone(),
        },
        DocumentAction::Purchase => DocumentActionMatchClauses::Purchase {
            original_document_clauses,
            owner_clause,
        },
    };

    let mut filter = DriveDocumentQueryFilter {
        contract,
        document_type_name: document_type_name.to_string(),
        action_clauses,
    };
    filter
        .canonicalize_clause_values(platform_version)
        .map_err(|e| e.to_string())?;
    let validation = filter.validate();
    if let Some(error) = validation.errors.first() {
        return Err(error.to_string());
    }
    Ok(filter.action_clauses)
}
