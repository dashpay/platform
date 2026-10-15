//! Filters validated and bound to their data contracts, and matching.

use super::participants::addresses;
use super::{
    check_filter_count, missing_action, DocumentAction, DocumentActionMatch, DocumentFilter, Role,
    StateTransitionFilter, SubscriptionFilterError, MAX_ACTIONS_PER_DOCUMENT_FILTER,
    MAX_ADDRESSES_PER_FILTER, MAX_CLAUSES_PER_ACTION, MAX_OWNER_IDS_PER_ACTION,
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
use dpp::state_transition::{StateTransition, StateTransitionOwned};
use dpp::version::PlatformVersion;
use drive::query::filter::{
    canonicalize_where_clause, DocumentActionMatchClauses, DriveDocumentQueryFilter,
    TransitionCheckResult,
};
use drive::query::{InternalClauses, ValueClause, WhereClause, WhereOperator};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Why a transition matched: the request filters it matched and, for a batch, the positions of
/// its document transitions that did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterMatch {
    /// Indexes of the matched filters, in request order.
    pub matched_filters: Vec<u32>,
    /// Positions, in the batch's transitions, of the document transitions that matched.
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
}

#[derive(Debug, Clone)]
struct ResolvedDocumentFilter {
    data_contract_id: Identifier,
    document_type_name: Option<String>,
    batch_owner_id: Option<Identifier>,
    actions: ResolvedActions,
}

#[derive(Debug, Clone)]
enum ResolvedActions {
    /// Every action.
    Any,
    /// These actions, on any document type of the contract.
    Kinds(BTreeSet<DocumentAction>),
    /// One Drive filter per action alternative, on the filter's document type.
    Clauses(Vec<DriveDocumentQueryFilter<Arc<DataContract>>>),
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
    /// `data_contract`; clause values are brought to their fields' types.
    pub fn resolve(
        filters: Vec<StateTransitionFilter>,
        data_contract: impl Fn(&Identifier) -> Option<Arc<DataContract>>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, SubscriptionFilterError> {
        check_filter_count(filters.len())?;
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
        // Read only when an address filter needs them.
        let mut named_addresses = None;
        let mut matched_filters = Vec::new();
        let mut matched_batch_positions = BTreeSet::new();

        for (index, filter) in self.filters.iter().enumerate() {
            let matched = match filter {
                ResolvedFilter::Addresses {
                    addresses: watched,
                    role,
                } => named_addresses
                    .get_or_insert_with(|| addresses(state_transition))
                    .iter()
                    .any(|(address, side)| role.admits(*side) && watched.contains(address)),
                ResolvedFilter::Documents(filter) => {
                    let StateTransition::Batch(batch) = state_transition else {
                        continue;
                    };
                    let batch_owner_id = batch.owner_id();
                    let mut matched = false;
                    for (position, transition) in batch.transitions_iter().enumerate() {
                        if let BatchedTransitionRef::Document(transition) = transition {
                            if filter.matches(transition, batch_owner_id, platform_version) {
                                matched = true;
                                matched_batch_positions.insert(position as u32);
                            }
                        }
                    }
                    matched
                }
            };
            if matched {
                matched_filters.push(index as u32);
            }
        }

        (!matched_filters.is_empty()).then(|| FilterMatch {
            matched_filters,
            matched_batch_positions: matched_batch_positions.into_iter().collect(),
        })
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
        if base.data_contract_id() != self.data_contract_id
            || self
                .document_type_name
                .as_ref()
                .is_some_and(|name| base.document_type_name() != name)
            || self
                .batch_owner_id
                .is_some_and(|owner| owner != batch_owner_id)
        {
            return false;
        }
        match &self.actions {
            ResolvedActions::Any => true,
            ResolvedActions::Kinds(actions) => actions.contains(&action_of(transition)),
            ResolvedActions::Clauses(alternatives) => {
                let batch_owner = Value::from(batch_owner_id);
                alternatives.iter().any(|filter| {
                    filter.matches_document_transition(
                        transition,
                        Some(&batch_owner),
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
    Ok(match filter {
        StateTransitionFilter::Documents(filter) => {
            let contract = data_contract(&filter.data_contract_id).ok_or(
                SubscriptionFilterError::DataContractNotFound(filter.data_contract_id),
            )?;
            ResolvedFilter::Documents(
                resolve_document_filter(filter, contract, platform_version).map_err(invalid)?,
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
    })
}

fn resolve_document_filter(
    filter: DocumentFilter,
    contract: Arc<DataContract>,
    platform_version: &PlatformVersion,
) -> Result<ResolvedDocumentFilter, String> {
    if filter.actions.len() > MAX_ACTIONS_PER_DOCUMENT_FILTER {
        return Err(format!(
            "at most {MAX_ACTIONS_PER_DOCUMENT_FILTER} actions are allowed, got {}",
            filter.actions.len()
        ));
    }
    if let Some(document_type_name) = &filter.document_type_name {
        if contract
            .document_type_optional_for_name(document_type_name)
            .is_none()
        {
            return Err(format!(
                "data contract {} has no document type '{document_type_name}'",
                contract.id()
            ));
        }
    }
    let actions = match &filter.document_type_name {
        _ if filter.actions.is_empty() => ResolvedActions::Any,
        None => {
            if filter
                .actions
                .iter()
                .any(DocumentActionMatch::has_constraints)
            {
                return Err("action clauses need a document type".to_string());
            }
            ResolvedActions::Kinds(
                filter
                    .actions
                    .iter()
                    .map(|action| action.action.ok_or_else(missing_action))
                    .collect::<Result<_, _>>()?,
            )
        }
        Some(document_type_name) => ResolvedActions::Clauses(
            filter
                .actions
                .iter()
                .map(|action| drive_filter(action, &contract, document_type_name, platform_version))
                .collect::<Result<_, _>>()?,
        ),
    };
    Ok(ResolvedDocumentFilter {
        data_contract_id: filter.data_contract_id,
        document_type_name: filter.document_type_name,
        batch_owner_id: filter.batch_owner_id,
        actions,
    })
}

/// The Drive filter for one action alternative, its clause values canonicalized and validated.
fn drive_filter(
    action: &DocumentActionMatch,
    contract: &Arc<DataContract>,
    document_type_name: &str,
    platform_version: &PlatformVersion,
) -> Result<DriveDocumentQueryFilter<Arc<DataContract>>, String> {
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
    if action.owner_ids.len() > MAX_OWNER_IDS_PER_ACTION {
        return Err(format!(
            "at most {MAX_OWNER_IDS_PER_ACTION} owner ids are allowed, got {}",
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
        .document_type_optional_for_name(document_type_name)
        .ok_or_else(|| format!("no document type '{document_type_name}'"))?;
    // Canonicalized before grouping, which pairs range bounds of one integer width only.
    let extract = |clauses: &[WhereClause]| {
        let mut clauses = clauses.to_vec();
        for clause in &mut clauses {
            canonicalize_where_clause(document_type, clause).map_err(|e| e.to_string())?;
        }
        InternalClauses::extract_from_clauses(clauses, platform_version).map_err(|e| e.to_string())
    };
    let new_document_clauses = extract(&action.new_document_where)?;
    let original_document_clauses = extract(&action.original_document_where)?;

    // Of a document before the transition, a transition carries only its `$id`, except an
    // indexOnly document's delete, which carries its values.
    let index_only_delete = kind == DocumentAction::Delete && document_type.index_only();
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
        contract: contract.clone(),
        document_type_name: document_type_name.to_string(),
        action_clauses,
    };
    filter
        .canonicalize_clause_values()
        .map_err(|e| e.to_string())?;
    if let Some(error) = filter.validate().errors.first() {
        return Err(error.to_string());
    }
    Ok(filter)
}
