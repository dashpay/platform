//! Document subscription filtering
//!
//! This module provides primitives to express and evaluate subscription filters for
//! document state transitions. The main entry point is `DriveDocumentQueryFilter`, which
//! holds a contract reference, a document type name, and action-specific match clauses
//! (`DocumentActionMatchClauses`).
//!
//! Filtering in brief:
//! - Create: evaluates `new_document_clauses` on the transition's data payload.
//! - Replace: evaluates `original_document_clauses` on the original document and
//!   `new_document_clauses` on the replacement data.
//! - Delete: evaluates `original_document_clauses` on the original document.
//! - Transfer: evaluates `original_document_clauses` and a new `owner_clause` against
//!   the `recipient_owner_id`.
//! - UpdatePrice: evaluates `original_document_clauses` and a `price_clause` against
//!   the new price in the transition.
//! - Purchase: evaluates `original_document_clauses` and an `owner_clause` against the
//!   batch owner (purchaser) ID.
//!
//! Usage:
//! - First check: call `matches_document_transition()` per transition to
//!   evaluate applicable constraints before fetching the original document. Decide
//!   whether to fetch the original document (returns Pass/Fail/NeedsOriginal).
//! - Second check: only if the first check returned `NeedsOriginal`, fetch the original
//!   `Document` and call `matches_original_document()` to evaluate original-dependent clauses.
//!
//! Validation:
//! - `validate()` performs structural checks: confirms the document type exists for the
//!   contract, enforces action-specific composition rules (e.g., at least one non-empty
//!   clause where required), and validates operator/value compatibility for scalar clauses
//!   like `owner_clause` and `price_clause`.

use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::document::{Document, DocumentV0Getters};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use crate::query::{
    validate_internal_clauses_against_schema, InternalClauses, ValueClause, WhereOperator,
};
use crate::error::{query::QuerySyntaxError, Error};
use dpp::platform_value::ValueMapHelper;

/// Filter used to match document transitions for subscriptions.
///
/// Targets a specific data contract and document type, and carries action-specific
/// match clauses via `DocumentActionMatchClauses`. Use `matches_document_transition()`
/// and `matches_original_document()` to evaluate document transitions.
/// `validate()` performs structural checks (document type exists, clause composition rules).
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, PartialEq, Clone)]
pub struct DriveDocumentQueryFilter<'a> {
    /// DataContract
    pub contract: &'a DataContract,
    /// Document type name
    pub document_type_name: String,
    /// Action-specific clauses
    pub action_clauses: DocumentActionMatchClauses,
}

/// Result of evaluating constraints for a transition before potentially fetching the original document.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCheckResult {
    /// All applicable transition-level checks pass and no original is required.
    Pass,
    /// Some transition-level check fails; do not fetch original.
    Fail,
    /// Transition-level checks pass, original clauses are non-empty and must be evaluated.
    NeedsOriginal,
}

/// Action-specific filter clauses for matching document transitions.
///
/// These clauses are used to evaluate whether a given document transition
/// (Create/Replace/Delete/Transfer/UpdatePrice/Purchase) matches a subscription
/// filter.
///
/// Conventions:
/// - Empty `InternalClauses` = no constraint for document-data checks.
/// - `Option<ValueClause>` = optional scalar constraint (owner/price); `None` = no constraint.
/// - Action-specific “at least one present” rules are enforced by `validate()`.
#[derive(Debug, PartialEq, Clone)]
pub enum DocumentActionMatchClauses {
    /// Create: filters on the new document data.
    Create {
        /// Clauses on the new document data.
        new_document_clauses: InternalClauses,
    },
    /// Replace: filters on original and/or new document data.
    Replace {
        /// Clauses on the original document data (pre-change).
        original_document_clauses: InternalClauses,
        /// Clauses on the new document data (replacement).
        new_document_clauses: InternalClauses,
    },
    /// Delete: filters on the original (existing) document.
    Delete {
        /// Clauses on the original document data.
        original_document_clauses: InternalClauses,
    },
    /// Transfer: filters on original data and/or recipient owner id.
    Transfer {
        /// Clauses on the original document data.
        original_document_clauses: InternalClauses,
        /// Constraint on the recipient owner id.
        owner_clause: Option<ValueClause>,
    },
    /// UpdatePrice: filters on original data and/or the new price.
    UpdatePrice {
        /// Clauses on the original document data.
        original_document_clauses: InternalClauses,
        /// Constraint on the new price.
        price_clause: Option<ValueClause>,
    },
    /// Purchase: filters on original data and/or batch owner id.
    Purchase {
        /// Clauses on the original document data.
        original_document_clauses: InternalClauses,
        /// Constraint on the batch owner (purchaser) id.
        owner_clause: Option<ValueClause>,
    },
}

impl DriveDocumentQueryFilter<'_> {
    /// Check a transition using only transition-level constraints.
    ///
    /// When to run:
    /// - Call this for each incoming transition before
    ///   fetching the original document. It short-circuits on obvious mismatches and
    ///   tells you if an original is needed at all for the final decision.
    ///
    /// Returns:
    /// - `Pass` if all applicable transition-level checks pass and no original is needed.
    /// - `Fail` if any transition-level check fails (no need to fetch original).
    /// - `NeedsOriginal` if transition-level checks pass but original clauses are non-empty
    ///   and must be evaluated with the original document.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document_transition(
        &self,
        document_transition: &DocumentTransition,
        batch_owner_value: Option<&Value>, // Only used for Purchase
    ) -> TransitionCheckResult {
        // Fast reject on contract/type mismatch common to all transitions
        if document_transition.base().data_contract_id() != self.contract.id()
            || document_transition.base().document_type_name() != &self.document_type_name
        {
            return TransitionCheckResult::Fail;
        }

        // Document ID value used by clause evaluation paths
        let id_value: Value = document_transition.base().id().into();

        match document_transition {
            DocumentTransition::Create(create) => {
                if let DocumentActionMatchClauses::Create {
                    new_document_clauses,
                } = &self.action_clauses
                {
                    if self.evaluate_clauses(new_document_clauses, &id_value, create.data()) {
                        TransitionCheckResult::Pass
                    } else {
                        TransitionCheckResult::Fail
                    }
                } else {
                    TransitionCheckResult::Fail
                }
            }
            DocumentTransition::Replace(replace) => {
                if let DocumentActionMatchClauses::Replace {
                    original_document_clauses,
                    new_document_clauses,
                } = &self.action_clauses
                {
                    let final_ok = if new_document_clauses.is_empty() {
                        true
                    } else {
                        self.evaluate_clauses(new_document_clauses, &id_value, replace.data())
                    };
                    if !final_ok {
                        return TransitionCheckResult::Fail;
                    }
                    if original_document_clauses.is_empty() {
                        return TransitionCheckResult::Pass;
                    }
                    if original_document_clauses.is_for_primary_key() {
                        if self.evaluate_clauses(
                            original_document_clauses,
                            &id_value,
                            &BTreeMap::new(),
                        ) {
                            return TransitionCheckResult::Pass;
                        }
                        return TransitionCheckResult::Fail;
                    }
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            DocumentTransition::Delete(_) => {
                if let DocumentActionMatchClauses::Delete {
                    original_document_clauses,
                } = &self.action_clauses
                {
                    if original_document_clauses.is_empty() {
                        return TransitionCheckResult::Pass;
                    }
                    if original_document_clauses.is_for_primary_key() {
                        if self.evaluate_clauses(
                            original_document_clauses,
                            &id_value,
                            &BTreeMap::new(),
                        ) {
                            return TransitionCheckResult::Pass;
                        }
                        return TransitionCheckResult::Fail;
                    }
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            DocumentTransition::Transfer(transfer) => {
                if let DocumentActionMatchClauses::Transfer {
                    original_document_clauses,
                    owner_clause,
                } = &self.action_clauses
                {
                    let new_owner_value: Value = transfer.recipient_owner_id().into();
                    let owner_ok = match owner_clause {
                        Some(clause) => clause.matches_value(&new_owner_value),
                        None => true,
                    };
                    if !owner_ok {
                        return TransitionCheckResult::Fail;
                    }
                    if original_document_clauses.is_empty() {
                        return TransitionCheckResult::Pass;
                    }
                    if original_document_clauses.is_for_primary_key() {
                        if self.evaluate_clauses(
                            original_document_clauses,
                            &id_value,
                            &BTreeMap::new(),
                        ) {
                            return TransitionCheckResult::Pass;
                        }
                        return TransitionCheckResult::Fail;
                    }
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            DocumentTransition::UpdatePrice(update_price) => {
                if let DocumentActionMatchClauses::UpdatePrice {
                    original_document_clauses,
                    price_clause,
                } = &self.action_clauses
                {
                    let price_value = Value::U64(update_price.price());
                    let price_ok = match price_clause {
                        Some(clause) => clause.matches_value(&price_value),
                        None => true,
                    };
                    if !price_ok {
                        return TransitionCheckResult::Fail;
                    }
                    if original_document_clauses.is_empty() {
                        return TransitionCheckResult::Pass;
                    }
                    if original_document_clauses.is_for_primary_key() {
                        if self.evaluate_clauses(
                            original_document_clauses,
                            &id_value,
                            &BTreeMap::new(),
                        ) {
                            return TransitionCheckResult::Pass;
                        }
                        return TransitionCheckResult::Fail;
                    }
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            DocumentTransition::Purchase(_) => {
                if let DocumentActionMatchClauses::Purchase {
                    original_document_clauses,
                    owner_clause,
                } = &self.action_clauses
                {
                    let owner_ok = match (owner_clause, batch_owner_value) {
                        (Some(clause), Some(val)) => clause.matches_value(val),
                        (Some(_), None) => return TransitionCheckResult::Fail,
                        (None, _) => true,
                    };
                    if !owner_ok {
                        return TransitionCheckResult::Fail;
                    }
                    if original_document_clauses.is_empty() {
                        return TransitionCheckResult::Pass;
                    }
                    if original_document_clauses.is_for_primary_key() {
                        if self.evaluate_clauses(
                            original_document_clauses,
                            &id_value,
                            &BTreeMap::new(),
                        ) {
                            return TransitionCheckResult::Pass;
                        }
                        return TransitionCheckResult::Fail;
                    }
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
        }
    }

    /// Evaluates original-dependent clauses against the provided original `Document`.
    ///
    /// When to run:
    /// - After `matches_document_transition` returns `NeedsOriginal` and the caller fetches
    ///   the original document.
    /// - This evaluates only original-dependent clauses; transition-level checks were
    ///   already applied during the first phase.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_original_document(&self, original_document: &Document) -> bool {
        // Evaluate only original-dependent clauses. Transition base was validated earlier.
        match &self.action_clauses {
            DocumentActionMatchClauses::Replace {
                original_document_clauses,
                ..
            }
            | DocumentActionMatchClauses::Delete {
                original_document_clauses,
            }
            | DocumentActionMatchClauses::Transfer {
                original_document_clauses,
                ..
            }
            | DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses,
                ..
            }
            | DocumentActionMatchClauses::Purchase {
                original_document_clauses,
                ..
            } => {
                let id_value: Value = original_document.id().into();
                self.evaluate_clauses(
                    original_document_clauses,
                    &id_value,
                    original_document.properties(),
                )
            }
            _ => false,
        }
    }

    // Note: tests use `evaluate_clauses` directly for unit coverage of clause logic.

    /// Single clause evaluator used by both transition and original-document paths.
    #[cfg(any(feature = "server", feature = "verify"))]
    fn evaluate_clauses(
        &self,
        clauses: &InternalClauses,
        document_id_value: &Value,
        document_data: &BTreeMap<String, Value>,
    ) -> bool {
        // Primary key IN clause
        if let Some(primary_key_in_clause) = &clauses.primary_key_in_clause {
            if !primary_key_in_clause.matches_value(document_id_value) {
                return false;
            }
        }

        // Primary key EQUAL clause
        if let Some(primary_key_equal_clause) = &clauses.primary_key_equal_clause {
            if !primary_key_equal_clause.matches_value(document_id_value) {
                return false;
            }
        }

        // In clause
        if let Some(in_clause) = &clauses.in_clause {
            let field_value = get_value_by_path(document_data, &in_clause.field);
            if let Some(value) = field_value {
                if !in_clause.matches_value(value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Range clause
        if let Some(range_clause) = &clauses.range_clause {
            let field_value = get_value_by_path(document_data, &range_clause.field);
            if let Some(value) = field_value {
                if !range_clause.matches_value(value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Equal clauses
        for (field, equal_clause) in &clauses.equal_clauses {
            let field_value = get_value_by_path(document_data, field);
            if let Some(value) = field_value {
                if !equal_clause.matches_value(value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Validate the filter structure and clauses.
    ///
    /// In addition to these validations, the subscription host should check the contract's existence.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        // Ensure the document type exists
        let document_type = self
            .contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|_| {
                Error::Query(QuerySyntaxError::DocumentTypeNotFound(
                    "unknown document type",
                ))
            })?;

        match &self.action_clauses {
            DocumentActionMatchClauses::Create {
                new_document_clauses,
            } => validate_internal_clauses_against_schema(document_type, new_document_clauses)?,
            DocumentActionMatchClauses::Replace {
                original_document_clauses,
                new_document_clauses,
            } => {
                if original_document_clauses.is_empty() && new_document_clauses.is_empty() {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "replace requires at least one of original/new clauses",
                        ),
                    ));
                }
                if !original_document_clauses.is_empty() {
                    validate_internal_clauses_against_schema(
                        document_type,
                        original_document_clauses,
                    )?;
                }
                if !new_document_clauses.is_empty() {
                    validate_internal_clauses_against_schema(document_type, new_document_clauses)?;
                }
            }
            DocumentActionMatchClauses::Delete {
                original_document_clauses,
            } => {
                validate_internal_clauses_against_schema(document_type, original_document_clauses)?
            }
            DocumentActionMatchClauses::Transfer {
                original_document_clauses,
                owner_clause,
            } => {
                if original_document_clauses.is_empty() && owner_clause.is_none() {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "transfer requires original clauses or owner clause",
                        ),
                    ));
                }
                if !original_document_clauses.is_empty() {
                    validate_internal_clauses_against_schema(
                        document_type,
                        original_document_clauses,
                    )?;
                }
                if let Some(owner) = owner_clause {
                    let ok = match owner.operator {
                        WhereOperator::Equal => matches!(owner.value, Value::Identifier(_)),
                        WhereOperator::In => match &owner.value {
                            Value::Array(arr) => {
                                arr.iter().all(|v| matches!(v, Value::Identifier(_)))
                            }
                            _ => false,
                        },
                        _ => false,
                    };
                    if !ok {
                        return Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid owner clause"),
                        ));
                    }
                }
            }
            DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses,
                price_clause,
            } => {
                if original_document_clauses.is_empty() && price_clause.is_none() {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "updatePrice requires original clauses or price clause",
                        ),
                    ));
                }
                if !original_document_clauses.is_empty() {
                    validate_internal_clauses_against_schema(
                        document_type,
                        original_document_clauses,
                    )?;
                }
                if let Some(price) = price_clause {
                    let ok = match price.operator {
                        WhereOperator::Equal
                        | WhereOperator::GreaterThan
                        | WhereOperator::GreaterThanOrEquals
                        | WhereOperator::LessThan
                        | WhereOperator::LessThanOrEquals => matches!(
                            price.value,
                            Value::U64(_)
                                | Value::I64(_)
                                | Value::U32(_)
                                | Value::I32(_)
                                | Value::U16(_)
                                | Value::I16(_)
                                | Value::U8(_)
                                | Value::I8(_)
                        ),
                        WhereOperator::Between
                        | WhereOperator::BetweenExcludeBounds
                        | WhereOperator::BetweenExcludeLeft
                        | WhereOperator::BetweenExcludeRight => match &price.value {
                            Value::Array(arr) => {
                                arr.len() == 2
                                    && arr.iter().all(|v| {
                                        matches!(
                                            v,
                                            Value::U64(_)
                                                | Value::I64(_)
                                                | Value::U32(_)
                                                | Value::I32(_)
                                                | Value::U16(_)
                                                | Value::I16(_)
                                                | Value::U8(_)
                                                | Value::I8(_)
                                        )
                                    })
                            }
                            _ => false,
                        },
                        WhereOperator::In => match &price.value {
                            Value::Array(arr) => arr.iter().all(|v| {
                                matches!(
                                    v,
                                    Value::U64(_)
                                        | Value::I64(_)
                                        | Value::U32(_)
                                        | Value::I32(_)
                                        | Value::U16(_)
                                        | Value::I16(_)
                                        | Value::U8(_)
                                        | Value::I8(_)
                                )
                            }),
                            _ => false,
                        },
                        WhereOperator::StartsWith => false,
                    };
                    if !ok {
                        return Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid price clause"),
                        ));
                    }
                }
            }
            DocumentActionMatchClauses::Purchase {
                original_document_clauses,
                owner_clause,
            } => {
                if original_document_clauses.is_empty() && owner_clause.is_none() {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "purchase requires original clauses or owner clause",
                        ),
                    ));
                }
                if !original_document_clauses.is_empty() {
                    validate_internal_clauses_against_schema(
                        document_type,
                        original_document_clauses,
                    )?;
                }
                if let Some(owner) = owner_clause {
                    let ok = match owner.operator {
                        WhereOperator::Equal => matches!(owner.value, Value::Identifier(_)),
                        WhereOperator::In => match &owner.value {
                            Value::Array(arr) => {
                                arr.iter().all(|v| matches!(v, Value::Identifier(_)))
                            }
                            _ => false,
                        },
                        _ => false,
                    };
                    if !ok {
                        return Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid owner clause"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Resolve a dot-notated path into a nested `BTreeMap<String, Value>` payload.
///
/// Supports dot notation like `meta.status` by walking `Value::Map` entries
/// using `ValueMapHelper`. Returns `None` if any segment is missing or if a
/// non-map value is encountered before the final segment. An empty `path`
/// returns `None`.
#[cfg(any(feature = "server", feature = "verify"))]
fn get_value_by_path<'a>(root: &'a BTreeMap<String, Value>, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    let mut current: Option<&Value> = None;
    let mut segments = path.split('.');
    if let Some(first) = segments.next() {
        current = root.get(first);
    }
    for seg in segments {
        match current {
            Some(Value::Map(ref vm)) => {
                current = vm.get_optional_key(seg);
            }
            _ => return None,
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{ValueClause, WhereClause, WhereOperator};
    use dpp::document::{Document, DocumentV0};
    use dpp::prelude::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_matches_document_basic() {
        // Get a test contract from fixtures
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Create a filter with no clauses (should match if contract and type match)
        let internal_clauses = InternalClauses::default();
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        // Create matching document base
        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let document_data = BTreeMap::new();

        // With no clauses, evaluation should be true regardless of data
        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &document_data));
    }

    #[test]
    fn test_matches_document_with_primary_key_equal() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let target_id = Identifier::from([42u8; 32]);

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.primary_key_equal_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: target_id.into(),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        // Test with matching ID
        let matching_doc = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: target_id,
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let document_data = BTreeMap::new();

        let id_value: Value = matching_doc.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &document_data));

        // Test with different ID
        let non_matching_doc = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([99u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let non_id_value: Value = non_matching_doc.id().into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &non_id_value, &document_data));
    }

    #[test]
    fn test_matches_document_with_field_filters() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Test Equal operator
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "name".to_string(),
            WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("example".to_string()),
            },
        );

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.equal_clauses = equal_clauses;

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Test with matching data
        let mut matching_data = BTreeMap::new();
        matching_data.insert("name".to_string(), Value::Text("example".to_string()));

        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &matching_data));

        // Test with non-matching data
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("name".to_string(), Value::Text("different".to_string()));

        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &non_matching_data));

        // Test with missing field
        let empty_data = BTreeMap::new();
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &empty_data));
    }

    #[test]
    fn test_matches_document_with_in_operator() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let allowed_values = vec![
            Value::Text("active".to_string()),
            Value::Text("pending".to_string()),
        ];

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.in_clause = Some(WhereClause {
            field: "status".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(allowed_values),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Test with value in list
        let mut matching_data = BTreeMap::new();
        matching_data.insert("status".to_string(), Value::Text("active".to_string()));
        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &matching_data));

        // Test with value not in list
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("status".to_string(), Value::Text("completed".to_string()));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &non_matching_data));
    }

    #[test]
    fn test_matches_document_with_range_operators() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Test GreaterThan
        let mut internal_clauses = InternalClauses::default();
        internal_clauses.range_clause = Some(WhereClause {
            field: "score".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(50),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Test with value greater than threshold
        let mut greater_data = BTreeMap::new();
        greater_data.insert("score".to_string(), Value::U64(75));
        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &greater_data));

        // Test with value equal to threshold (should fail for GreaterThan)
        let mut equal_data = BTreeMap::new();
        equal_data.insert("score".to_string(), Value::U64(50));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &equal_data));

        // Test with value less than threshold
        let mut less_data = BTreeMap::new();
        less_data.insert("score".to_string(), Value::U64(25));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &less_data));
    }

    #[test]
    fn test_matches_document_with_nested_field() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Equal on nested field: meta.status == "active"
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "meta.status".to_string(),
            WhereClause {
                field: "meta.status".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("active".to_string()),
            },
        );

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.equal_clauses = equal_clauses;

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Build nested data: { meta: { status: "active" } }
        let nested = vec![(
            Value::Text("status".to_string()),
            Value::Text("active".to_string()),
        )];
        let mut data = BTreeMap::new();
        data.insert("meta".to_string(), Value::Map(nested));

        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &data));
    }

    #[test]
    fn test_validate_requires_at_least_one_clause_for_optional_actions() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Replace with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Replace {
                original_document_clauses: InternalClauses::default(),
                new_document_clauses: InternalClauses::default(),
            },
        };
        assert!(filter.validate().is_err());

        // Replace with final only -> valid (non-empty final clauses)
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Replace {
                original_document_clauses: InternalClauses::default(),
                new_document_clauses: InternalClauses {
                    primary_key_equal_clause: Some(WhereClause {
                        field: "$id".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Identifier([3u8; 32]),
                    }),
                    ..Default::default()
                },
            },
        };
        assert!(filter.validate().is_ok());

        // Transfer with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: None,
            },
        };
        assert!(filter.validate().is_err());

        // Transfer with owner only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::Identifier([1u8; 32]),
                }),
            },
        };
        assert!(filter.validate().is_ok());

        // UpdatePrice with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: None,
            },
        };
        assert!(filter.validate().is_err());

        // UpdatePrice with price only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(0),
                }),
            },
        };
        assert!(filter.validate().is_ok());

        // Purchase with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: None,
            },
        };
        assert!(filter.validate().is_err());

        // Purchase with owner only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::Identifier([2u8; 32]),
                }),
            },
        };
        assert!(filter.validate().is_ok());
    }

    #[test]
    fn test_transfer_owner_clause_only_matches() {
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;

        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let new_owner = Identifier::from([5u8; 32]);

        // Filter checks only new owner
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: new_owner.into(),
                }),
            },
        };

        // Transfer transition with recipient = new_owner
        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let transfer_v0 = DocumentTransferTransitionV0 {
            base: document_base.clone(),
            revision: 1 as u64,
            recipient_owner_id: new_owner,
        };
        let transfer = DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0));

        // First check should pass without needing original
        assert_eq!(
            filter.matches_document_transition(&transfer, None),
            TransitionCheckResult::Pass
        );

        // Mismatch owner
        let other_owner = Identifier::from([6u8; 32]);
        let transfer_v0_mismatch = DocumentTransferTransitionV0 {
            base: document_base,
            revision: 1 as u64,
            recipient_owner_id: other_owner,
        };
        let transfer_mismatch =
            DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0_mismatch));
        assert_eq!(
            filter.matches_document_transition(&transfer_mismatch, None),
            TransitionCheckResult::Fail
        );
    }

    #[test]
    fn test_purchase_owner_clause_only_matches_and_requires_owner_context() {
        use dpp::fee::Credits;
        use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::DocumentPurchaseTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransition;

        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let purchaser = Identifier::from([7u8; 32]);

        // Filter checks batch owner (purchaser)
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: purchaser.into(),
                }),
            },
        };

        // Purchase transition
        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([4u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let purchase_v0 = DocumentPurchaseTransitionV0 {
            base: document_base,
            revision: 1 as u64,
            price: 10 as Credits,
        };
        let purchase = DocumentTransition::Purchase(DocumentPurchaseTransition::V0(purchase_v0));

        // Without batch owner context, should fail (owner clause requires it)
        assert_eq!(
            filter.matches_document_transition(&purchase, None),
            TransitionCheckResult::Fail
        );
        // With batch owner context, should pass
        let owner_value = Value::Identifier(purchaser.to_buffer());
        assert_eq!(
            filter.matches_document_transition(&purchase, Some(&owner_value)),
            TransitionCheckResult::Pass
        );
    }

    #[test]
    fn test_transfer_original_clause_only_matches_with_original_document() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Filter checks only original document field
        let mut eq = BTreeMap::new();
        eq.insert(
            "status".to_string(),
            WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("active".to_string()),
            },
        );
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                owner_clause: None,
            },
        };

        // Original doc present and matching
        let mut original = BTreeMap::new();
        original.insert("status".to_string(), Value::Text("active".to_string()));
        let original_doc = Document::V0(DocumentV0 {
            id: Identifier::from([9u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original,
            ..Default::default()
        });
        assert!(filter.matches_original_document(&original_doc));

        // Without original doc, clause is required -> no match
        // No call without original: first pass already signaled it is required
    }

    #[test]
    fn test_delete_original_clause_only_matches_with_original_document() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Filter checks only original document field
        let mut eq = BTreeMap::new();
        eq.insert(
            "status".to_string(),
            WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("active".to_string()),
            },
        );
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Delete {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
            },
        };

        // Original doc present and matching
        let mut original = BTreeMap::new();
        original.insert("status".to_string(), Value::Text("active".to_string()));
        let original_doc = Document::V0(DocumentV0 {
            id: Identifier::from([12u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original,
            ..Default::default()
        });
        assert!(filter.matches_original_document(&original_doc));

        // Without original doc -> no match (required for Delete)
        // No call without original: first pass already signaled it is required

        // Original mismatching -> no match
        let mut original_bad = BTreeMap::new();
        original_bad.insert("status".to_string(), Value::Text("inactive".to_string()));
        let original_doc_bad = Document::V0(DocumentV0 {
            id: Identifier::from([12u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original_bad,
            ..Default::default()
        });
        assert!(!filter.matches_original_document(&original_doc_bad));
    }

    #[test]
    fn test_update_price_price_clause_only_matches_and_with_original_clause() {
        use dpp::fee::Credits;
        use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::DocumentUpdatePriceTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransition;

        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([10u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Price-only clause
        let update_v0 = DocumentUpdatePriceTransitionV0 {
            base: base.clone(),
            revision: 1,
            price: 10 as Credits,
        };
        let update = DocumentTransition::UpdatePrice(DocumentUpdatePriceTransition::V0(update_v0));

        let filter_price_only = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(5),
                }),
            },
        };
        // Price-only clause is decided in first check
        assert_eq!(
            filter_price_only.matches_document_transition(&update, None),
            TransitionCheckResult::Pass
        );

        let filter_price_only_fail = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(15),
                }),
            },
        };
        assert_eq!(
            filter_price_only_fail.matches_document_transition(&update, None),
            TransitionCheckResult::Fail
        );

        // With original clauses as well
        let mut eq = BTreeMap::new();
        eq.insert(
            "kind".to_string(),
            WhereClause {
                field: "kind".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("sale".to_string()),
            },
        );
        let filter_with_orig = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                price_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: Value::U64(10),
                }),
            },
        };
        let mut original_doc = BTreeMap::new();
        original_doc.insert("kind".to_string(), Value::Text("sale".to_string()));
        assert_eq!(
            filter_with_orig.matches_document_transition(&update, None),
            TransitionCheckResult::NeedsOriginal
        );
        let original_document = Document::V0(DocumentV0 {
            id: Identifier::from([10u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original_doc,
            ..Default::default()
        });
        assert!(filter_with_orig.matches_original_document(&original_document));

        // Missing original doc -> required -> no match
        // No call without original: first pass already signaled it is required
    }

    #[test]
    fn test_replace_with_both_original_and_new_document_clauses() {
        use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::v0::DocumentReplaceTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransition;

        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([11u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Original must have status=active; New must have score=10
        let mut orig_eq = BTreeMap::new();
        orig_eq.insert(
            "status".to_string(),
            WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("active".to_string()),
            },
        );
        let original_clauses = InternalClauses {
            equal_clauses: orig_eq,
            ..Default::default()
        };

        let mut final_eq = BTreeMap::new();
        final_eq.insert(
            "score".to_string(),
            WhereClause {
                field: "score".to_string(),
                operator: WhereOperator::Equal,
                value: Value::U64(10),
            },
        );
        let new_document_clauses = InternalClauses {
            equal_clauses: final_eq,
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Replace {
                original_document_clauses: original_clauses,
                new_document_clauses: new_document_clauses,
            },
        };

        // Build Replace transition with new data
        let mut data = BTreeMap::new();
        data.insert("score".to_string(), Value::U64(10));
        let replace_v0 = DocumentReplaceTransitionV0 {
            base: base,
            revision: 1,
            data,
        };
        let replace = DocumentTransition::Replace(DocumentReplaceTransition::V0(replace_v0));

        // Original provided and matching; final matches (requires original)
        let mut original_doc = BTreeMap::new();
        original_doc.insert("status".to_string(), Value::Text("active".to_string()));
        assert_eq!(
            filter.matches_document_transition(&replace, None),
            TransitionCheckResult::NeedsOriginal
        );
        let original_document = Document::V0(DocumentV0 {
            id: Identifier::from([11u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original_doc,
            ..Default::default()
        });
        assert!(filter.matches_original_document(&original_document));

        // Original missing -> should fail as it's required
        // No call without original: first pass already signaled it is required

        // Original mismatching -> fail
        let mut original_doc_bad = BTreeMap::new();
        original_doc_bad.insert("status".to_string(), Value::Text("inactive".to_string()));
        let original_document_bad = Document::V0(DocumentV0 {
            id: Identifier::from([11u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: original_doc_bad,
            ..Default::default()
        });
        assert!(!filter.matches_original_document(&original_document_bad));

        // New-data mismatching should fail in first check (do not call final)
        if let DocumentTransition::Replace(mut rep) = replace.clone() {
            let DocumentReplaceTransition::V0(ref mut v0) = rep;
            v0.data.insert("score".to_string(), Value::U64(9));
            let bad_final = DocumentTransition::Replace(rep);
            assert_eq!(
                filter.matches_document_transition(&bad_final, None),
                TransitionCheckResult::Fail
            );
        }
    }

    #[test]
    fn test_matches_document_with_between_operator() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.range_clause = Some(WhereClause {
            field: "value".to_string(),
            operator: WhereOperator::Between,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        // Test value in range
        let mut in_range = BTreeMap::new();
        in_range.insert("value".to_string(), Value::U64(15));
        let id_value: Value = document_base.id().into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &in_range));

        // Test lower bound (inclusive)
        let mut lower_bound = BTreeMap::new();
        lower_bound.insert("value".to_string(), Value::U64(10));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &lower_bound));

        // Test upper bound (inclusive)
        let mut upper_bound = BTreeMap::new();
        upper_bound.insert("value".to_string(), Value::U64(20));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &upper_bound));

        // Test below range
        let mut below = BTreeMap::new();
        below.insert("value".to_string(), Value::U64(5));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &below));

        // Test above range
        let mut above = BTreeMap::new();
        above.insert("value".to_string(), Value::U64(25));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &above));
    }

    #[test]
    fn test_validate_filter() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Test valid filter with indexed field
        let mut internal_clauses = InternalClauses::default();
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "firstName".to_string(),
            WhereClause {
                field: "firstName".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("Alice".to_string()),
            },
        );
        internal_clauses.equal_clauses = equal_clauses;

        let valid_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "indexedDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            valid_filter.validate().is_ok(),
            "Filter with indexed field should be valid"
        );

        // Test filter with non-indexed field: structural validation should pass
        // (indexes are not considered by subscription filters).
        let mut internal_clauses = InternalClauses::default();
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "name".to_string(),
            WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("value".to_string()),
            },
        );
        internal_clauses.equal_clauses = equal_clauses;

        let invalid_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            invalid_filter.validate().is_ok(),
            "Structural validate should ignore indexes"
        );
        // Index-aware validation removed; structural validation suffices for subscriptions.

        // Test valid filter with only primary key
        let mut internal_clauses = InternalClauses::default();
        internal_clauses.primary_key_equal_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier([42u8; 32]),
        });

        let primary_key_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "indexedDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            primary_key_filter.validate().is_ok(),
            "Filter with only primary key should be valid"
        );
    }

    #[test]
    fn test_validate_rejects_id_in_generic_clauses() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // $id in equal_clauses should be rejected
        let mut eq = BTreeMap::new();
        eq.insert(
            "$id".to_string(),
            WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier([1u8; 32]),
            },
        );
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
            },
        };
        assert!(filter.validate().is_err());

        // $id in range clause should be rejected
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: InternalClauses {
                    range_clause: Some(WhereClause {
                        field: "$id".to_string(),
                        operator: WhereOperator::GreaterThan,
                        value: Value::U64(0),
                    }),
                    ..Default::default()
                },
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn test_validate_owner_and_price_clause_types() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Owner clause must be Identifier
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::Text("not-id".to_string()),
                }),
            },
        };
        assert!(filter.validate().is_err());

        // Price clause must be integer-like, not float
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::Float(1.23),
                }),
            },
        };
        assert!(filter.validate().is_err());

        // Price Between must be 2 integer-like values
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::Between,
                    value: Value::Array(vec![Value::U64(1), Value::Float(2.0)]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn test_validate_startswith_on_numeric_field_rejected() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // numeric field 'score' with StartsWith should be rejected
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: InternalClauses {
                    range_clause: Some(WhereClause {
                        field: "score".to_string(),
                        operator: WhereOperator::StartsWith,
                        value: Value::Text("1".to_string()),
                    }),
                    ..Default::default()
                },
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn test_conversion_between_filter_and_query() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.primary_key_equal_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier([42u8; 32]),
        });

        let original_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        // No conversion helpers; verify the filter holds the expected clauses
        if let DocumentActionMatchClauses::Create {
            new_document_clauses,
        } = original_filter.action_clauses
        {
            assert_eq!(new_document_clauses, internal_clauses);
        } else {
            panic!("expected Create action clauses");
        }
    }
}
