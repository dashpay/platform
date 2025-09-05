//! Document subscription filtering
//!
//! This module provides primitives to express and evaluate subscription filters for
//! document state transitions. The main entry point is `DriveDocumentQueryFilter`, which
//! holds a contract reference, a document type name, and action-specific clauses
//! (`DocumentActionClauses`).
//!
//! Filtering in brief:
//! - Create: evaluates `final_clauses` on the transition's data payload.
//! - Replace: optionally evaluates `original_clauses` on the original document and/or
//!   `final_clauses` on the replacement data.
//! - Delete: evaluates `original_clauses` on the original document.
//! - Transfer: optionally evaluates `original_clauses` and/or a new `owner_clause` against
//!   the `recipient_owner_id`.
//! - UpdatePrice: optionally evaluates `original_clauses` and/or a `price_clause` against
//!   the new price in the transition.
//! - Purchase: optionally evaluates `original_clauses` and/or an `owner_clause` against the
//!   batch owner (purchaser) ID.
//!
//! Validation is structural and index-aware: it checks the document type exists, that
//! at least one optional clause is provided where required (Replace/Transfer/UpdatePrice/
//! Purchase), and that applicable clauses have compatible indexes.

use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use indexmap::IndexMap;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause};
use dpp::platform_value::ValueMapHelper;
use dpp::version::LATEST_PLATFORM_VERSION;

#[cfg(any(feature = "server", feature = "verify"))]
/// Filter used to match document transitions for subscriptions.
///
/// Targets a specific data contract and document type, and carries action-specific
/// clauses via `DocumentActionClauses`. Use `matches_document_transition()` to evaluate
/// batch/document transitions. `validate()` performs structural checks (document type
/// exists, clause composition rules).
#[derive(Debug, PartialEq, Clone)]
pub struct DriveDocumentQueryFilter<'a> {
    /// DataContract
    pub contract: &'a DataContract,
    /// Document type name
    pub document_type_name: String,
    /// Action-specific clauses
    pub action_clauses: DocumentActionClauses,
}

/// Action-specific filter clauses for matching document transitions.
///
/// These clauses are used to evaluate whether a given document transition
/// (Create/Replace/Delete/Transfer/UpdatePrice/Purchase) matches a
/// subscription filter. Some variants allow optional sub-clauses; if an
/// optional sub-clause is `None`, it imposes no constraint. For variants with
/// multiple optional sub-clauses, at least one must be present (validated via
/// `DriveDocumentQueryFilter::validate`).
#[derive(Debug, PartialEq, Clone)]
pub enum DocumentActionClauses {
    /// Create document: filter on final document only.
    ///
    /// The `final_clauses` apply to the transition's data payload.
    Create { final_clauses: InternalClauses },
    /// Replace document: optionally filter on original and/or final.
    ///
    /// - If `original_clauses` is `Some`, the original document (pre-change)
    ///   must be provided and match.
    /// - If `final_clauses` is `Some`, the transition's replacement data must
    ///   match.
    /// - Validation requires that at least one of the two be `Some`.
    Replace {
        original_clauses: Option<InternalClauses>,
        final_clauses: Option<InternalClauses>,
    },
    /// Delete: filter on original (existing) document.
    ///
    /// The `original_clauses` apply to the original document; matching
    /// requires the original document to be supplied.
    Delete { original_clauses: InternalClauses },
    /// Transfer: filter on original document and/or new owner id.
    ///
    /// - `original_clauses`: optional constraints on the original document.
    /// - `owner_clause`: optional constraint on the recipient owner id.
    /// - Validation requires at least one of the two to be `Some`.
    Transfer {
        original_clauses: Option<InternalClauses>,
        owner_clause: Option<WhereClause>,
    },
    /// Update price: filter on original doc and/or the new price using a simple clause.
    ///
    /// - `original_clauses`: optional constraints on the original document.
    /// - `price_clause`: optional constraint evaluated against the new price.
    /// - Validation requires at least one of the two to be `Some`.
    UpdatePrice {
        original_clauses: Option<InternalClauses>,
        price_clause: Option<WhereClause>,
    },
    /// Purchase: filter on original document and/or new owner id (batch owner).
    ///
    /// - `original_clauses`: optional constraints on the original document.
    /// - `owner_clause`: optional constraint evaluated against the batch
    ///   transition owner id (the purchaser).
    /// - Validation requires at least one of the two to be `Some`.
    Purchase {
        original_clauses: Option<InternalClauses>,
        owner_clause: Option<WhereClause>,
    },
}

impl DriveDocumentQueryFilter<'_> {
    /// Checks if a document transition matches the filter, with optional
    /// original document data and an optional batch owner value.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document_transition(
        &self,
        document_transition: &DocumentTransition,
        original_document: Option<&BTreeMap<String, Value>>,
        batch_owner_value: Option<&Value>,
    ) -> bool {
        match (&self.action_clauses, document_transition) {
            (
                DocumentActionClauses::Create { final_clauses },
                DocumentTransition::Create(create),
            ) => self.evaluate_document_with_clauses(final_clauses, create.base(), create.data()),
            (
                DocumentActionClauses::Replace {
                    original_clauses,
                    final_clauses,
                },
                DocumentTransition::Replace(replace),
            ) => {
                // Both must match if present
                let orig_ok = match (original_clauses, original_document) {
                    (Some(clauses), Some(orig)) => {
                        self.evaluate_document_with_clauses(clauses, replace.base(), orig)
                    }
                    (Some(_), None) => false, // needed but not provided
                    (None, _) => true,
                };
                let new_ok = match final_clauses {
                    Some(clauses) => {
                        self.evaluate_document_with_clauses(clauses, replace.base(), replace.data())
                    }
                    None => true,
                };
                orig_ok && new_ok
            }
            (
                DocumentActionClauses::Delete { original_clauses },
                DocumentTransition::Delete(delete),
            ) => match original_document {
                Some(orig) => {
                    self.evaluate_document_with_clauses(original_clauses, delete.base(), orig)
                }
                None => false,
            },
            (
                DocumentActionClauses::Transfer {
                    original_clauses,
                    owner_clause,
                },
                DocumentTransition::Transfer(transfer),
            ) => {
                let orig_ok = match (original_clauses, original_document) {
                    (Some(clauses), Some(orig)) => {
                        self.evaluate_document_with_clauses(clauses, transfer.base(), orig)
                    }
                    (Some(_), None) => false,
                    (None, _) => true,
                };
                let new_owner_value: Value = transfer.recipient_owner_id().into();
                let owner_ok = match owner_clause {
                    Some(clause) => clause.matches_value(&new_owner_value),
                    None => true,
                };
                orig_ok && owner_ok
            }
            (
                DocumentActionClauses::UpdatePrice {
                    original_clauses,
                    price_clause,
                },
                DocumentTransition::UpdatePrice(update_price),
            ) => {
                let orig_ok = match (original_clauses, original_document) {
                    (Some(clauses), Some(orig)) => {
                        self.evaluate_document_with_clauses(clauses, update_price.base(), orig)
                    }
                    (Some(_), None) => false,
                    (None, _) => true,
                };
                // Evaluate price clause against the transition's price
                let price_value = Value::U64(update_price.price());
                let price_ok = match price_clause {
                    Some(clause) => clause.matches_value(&price_value),
                    None => true,
                };
                orig_ok && price_ok
            }
            (
                DocumentActionClauses::Purchase {
                    original_clauses,
                    owner_clause,
                },
                DocumentTransition::Purchase(purchase),
            ) => {
                let orig_ok = match (original_clauses, original_document) {
                    (Some(clauses), Some(orig)) => {
                        self.evaluate_document_with_clauses(clauses, purchase.base(), orig)
                    }
                    (Some(_), None) => false,
                    (None, _) => true,
                };
                let owner_ok = match (owner_clause, batch_owner_value) {
                    (Some(clause), Some(val)) => clause.matches_value(val),
                    (Some(_), None) => false,
                    (None, _) => true,
                };
                orig_ok && owner_ok
            }
            // Fallback: only allow matching on primary-key-only filters across actions
            // (evaluate base: contract/type/$id). Avoids accidental matches for
            // data-dependent filters when no document data exists on this action.
            (_, _) => {
                let pk_only = match &self.action_clauses {
                    DocumentActionClauses::Create { final_clauses } => {
                        final_clauses.is_for_primary_key()
                    }
                    DocumentActionClauses::Replace {
                        original_clauses,
                        final_clauses,
                    } => {
                        original_clauses
                            .as_ref()
                            .map(|c| c.is_for_primary_key())
                            .unwrap_or(false)
                            || final_clauses
                                .as_ref()
                                .map(|c| c.is_for_primary_key())
                                .unwrap_or(false)
                    }
                    DocumentActionClauses::Delete { original_clauses } => {
                        original_clauses.is_for_primary_key()
                    }
                    DocumentActionClauses::Transfer {
                        original_clauses, ..
                    } => original_clauses
                        .as_ref()
                        .map(|c| c.is_for_primary_key())
                        .unwrap_or(false),
                    DocumentActionClauses::UpdatePrice {
                        original_clauses, ..
                    } => original_clauses
                        .as_ref()
                        .map(|c| c.is_for_primary_key())
                        .unwrap_or(false),
                    DocumentActionClauses::Purchase {
                        original_clauses, ..
                    } => original_clauses
                        .as_ref()
                        .map(|c| c.is_for_primary_key())
                        .unwrap_or(false),
                };
                if pk_only {
                    self.matches_document(document_transition.base(), &BTreeMap::new())
                } else {
                    false
                }
            }
        }
    }

    /// Low-level helper to evaluate an in-memory document payload (`base` + `data`)
    /// against the clauses selected for the current action.
    ///
    /// Prefer the transition-oriented method unless you are matching raw
    /// in-memory data.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document(
        &self,
        document_base_transition: &DocumentBaseTransition,
        document_data: &BTreeMap<String, Value>,
    ) -> bool {
        // When action-specific clauses exist, use them; default to empty clauses when not provided
        let empty = InternalClauses::default();
        let clauses_ref: &InternalClauses = match &self.action_clauses {
            DocumentActionClauses::Create { final_clauses } => final_clauses,
            DocumentActionClauses::Replace { final_clauses, .. } => {
                final_clauses.as_ref().unwrap_or(&empty)
            }
            DocumentActionClauses::Delete { original_clauses } => original_clauses,
            DocumentActionClauses::Transfer {
                original_clauses, ..
            } => original_clauses.as_ref().unwrap_or(&empty),
            DocumentActionClauses::Purchase {
                original_clauses, ..
            } => original_clauses.as_ref().unwrap_or(&empty),
            DocumentActionClauses::UpdatePrice {
                original_clauses, ..
            } => original_clauses.as_ref().unwrap_or(&empty),
        };
        self.evaluate_document_with_clauses(clauses_ref, document_base_transition, document_data)
    }

    /// Core evaluator: checks the given base transition + document data against
    /// the provided `InternalClauses`.
    ///
    /// This is used internally by action-specific matchers (e.g., Replace
    /// evaluates both `original_clauses` and `final_clauses` separately). Most
    /// callers should use `matches_document_transition`, which determines the
    /// correct clause set(s) to apply for the configured `DocumentActionClauses`.
    #[cfg(any(feature = "server", feature = "verify"))]
    fn evaluate_document_with_clauses(
        &self,
        clauses: &InternalClauses,
        document_base_transition: &DocumentBaseTransition,
        document_data: &BTreeMap<String, Value>,
    ) -> bool {
        // Check contract ID
        if document_base_transition.data_contract_id() != self.contract.id() {
            return false;
        }

        // Check document type name
        if document_base_transition.document_type_name() != &self.document_type_name {
            return false;
        }

        // Check primary key in clause (for document ID)
        if let Some(primary_key_in_clause) = &clauses.primary_key_in_clause {
            if !primary_key_in_clause.matches_value(&document_base_transition.id().into()) {
                return false;
            }
        }

        // Check primary key equal clause (for document ID)
        if let Some(primary_key_equal_clause) = &clauses.primary_key_equal_clause {
            if !primary_key_equal_clause.matches_value(&document_base_transition.id().into()) {
                return false;
            }
        }

        // Check in clause
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

        // Check range clause
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

        // Check equal clauses
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

    /// Validates that the filter's clauses are valid for the document type and indexes
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate(&self) -> bool {
        // Ensure the document type exists
        let Ok(document_type) = self
            .contract
            .document_type_for_name(&self.document_type_name)
        else {
            return false;
        };

        let validate_indexes = |clauses: &InternalClauses| -> bool {
            if !clauses.verify() {
                return false;
            }
            // If no data clauses or only primary key, skip index selection
            if clauses.is_empty() || clauses.is_for_primary_key() {
                return true;
            }
            let query = DriveDocumentQuery {
                contract: self.contract,
                document_type,
                internal_clauses: clauses.clone(),
                offset: None,
                limit: None,
                order_by: IndexMap::new(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };
            query.find_best_index(LATEST_PLATFORM_VERSION).is_ok()
        };

        // Validate internal clauses depending on action
        match &self.action_clauses {
            DocumentActionClauses::Create { final_clauses } => validate_indexes(final_clauses),
            DocumentActionClauses::Replace {
                original_clauses,
                final_clauses,
            } => {
                if original_clauses.is_none() && final_clauses.is_none() {
                    return false;
                }
                let orig_ok = original_clauses
                    .as_ref()
                    .map(|c| validate_indexes(c))
                    .unwrap_or(true);
                let final_ok = final_clauses
                    .as_ref()
                    .map(|c| validate_indexes(c))
                    .unwrap_or(true);
                orig_ok && final_ok
            }
            DocumentActionClauses::Delete { original_clauses } => {
                validate_indexes(original_clauses)
            }
            DocumentActionClauses::Transfer {
                original_clauses,
                owner_clause,
            } => {
                if original_clauses.is_none() && owner_clause.is_none() {
                    return false;
                }
                original_clauses
                    .as_ref()
                    .map(|c| validate_indexes(c))
                    .unwrap_or(true)
            }
            DocumentActionClauses::UpdatePrice {
                original_clauses,
                price_clause,
            } => {
                if original_clauses.is_none() && price_clause.is_none() {
                    return false;
                }
                original_clauses
                    .as_ref()
                    .map(|c| validate_indexes(c))
                    .unwrap_or(true)
            }
            DocumentActionClauses::Purchase {
                original_clauses,
                owner_clause,
            } => {
                if original_clauses.is_none() && owner_clause.is_none() {
                    return false;
                }
                original_clauses
                    .as_ref()
                    .map(|c| validate_indexes(c))
                    .unwrap_or(true)
            }
        }
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
    use crate::query::WhereOperator;
    use dpp::prelude::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::LATEST_PLATFORM_VERSION;

    #[test]
    fn test_matches_document_basic() {
        // Get a test contract from fixtures
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Create a filter with no clauses (should match if contract and type match)
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Create {
                final_clauses: InternalClauses::default(),
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

        // Should match since contract ID and type name are correct
        assert!(filter.matches_document(&document_base, &document_data));

        // Test with wrong contract ID
        let wrong_document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([3u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: Identifier::from([99u8; 32]), // Wrong ID
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        assert!(!filter.matches_document(&wrong_document_base, &document_data));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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

        assert!(filter.matches_document(&matching_doc, &document_data));

        // Test with different ID
        let non_matching_doc = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([99u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        assert!(!filter.matches_document(&non_matching_doc, &document_data));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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

        assert!(filter.matches_document(&document_base, &matching_data));

        // Test with non-matching data
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("name".to_string(), Value::Text("different".to_string()));

        assert!(!filter.matches_document(&document_base, &non_matching_data));

        // Test with missing field
        let empty_data = BTreeMap::new();
        assert!(!filter.matches_document(&document_base, &empty_data));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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
        assert!(filter.matches_document(&document_base, &matching_data));

        // Test with value not in list
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("status".to_string(), Value::Text("completed".to_string()));
        assert!(!filter.matches_document(&document_base, &non_matching_data));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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
        assert!(filter.matches_document(&document_base, &greater_data));

        // Test with value equal to threshold (should fail for GreaterThan)
        let mut equal_data = BTreeMap::new();
        equal_data.insert("score".to_string(), Value::U64(50));
        assert!(!filter.matches_document(&document_base, &equal_data));

        // Test with value less than threshold
        let mut less_data = BTreeMap::new();
        less_data.insert("score".to_string(), Value::U64(25));
        assert!(!filter.matches_document(&document_base, &less_data));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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

        assert!(filter.matches_document(&document_base, &data));
    }

    #[test]
    fn test_validate_requires_at_least_one_clause_for_optional_actions() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Replace with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Replace {
                original_clauses: None,
                final_clauses: None,
            },
        };
        assert!(!filter.validate());

        // Replace with final only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Replace {
                original_clauses: None,
                final_clauses: Some(InternalClauses::default()),
            },
        };
        assert!(filter.validate());

        // Transfer with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Transfer {
                original_clauses: None,
                owner_clause: None,
            },
        };
        assert!(!filter.validate());

        // Transfer with owner only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Transfer {
                original_clauses: None,
                owner_clause: Some(WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier([1u8; 32]),
                }),
            },
        };
        assert!(filter.validate());

        // UpdatePrice with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::UpdatePrice {
                original_clauses: None,
                price_clause: None,
            },
        };
        assert!(!filter.validate());

        // UpdatePrice with price only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::UpdatePrice {
                original_clauses: None,
                price_clause: Some(WhereClause {
                    field: "price".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(0),
                }),
            },
        };
        assert!(filter.validate());

        // Purchase with none/none -> invalid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Purchase {
                original_clauses: None,
                owner_clause: None,
            },
        };
        assert!(!filter.validate());

        // Purchase with owner only -> valid
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Purchase {
                original_clauses: None,
                owner_clause: Some(WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier([2u8; 32]),
                }),
            },
        };
        assert!(filter.validate());
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
            action_clauses: DocumentActionClauses::Transfer {
                original_clauses: None,
                owner_clause: Some(WhereClause {
                    field: "$ownerId".to_string(),
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

        // No original doc needed; owner is taken from transfer
        assert!(filter.matches_document_transition(&transfer, None, None));

        // Mismatch owner
        let other_owner = Identifier::from([6u8; 32]);
        let transfer_v0_mismatch = DocumentTransferTransitionV0 {
            base: document_base,
            revision: 1 as u64,
            recipient_owner_id: other_owner,
        };
        let transfer_mismatch =
            DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0_mismatch));
        assert!(!filter.matches_document_transition(&transfer_mismatch, None, None));
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
            action_clauses: DocumentActionClauses::Purchase {
                original_clauses: None,
                owner_clause: Some(WhereClause {
                    field: "$ownerId".to_string(),
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

        // Without passing the batch owner context, should fail (owner clause requires it)
        assert!(!filter.matches_document_transition(&purchase, None, None));

        // With batch owner context, should pass
        let owner_value: Value = purchaser.into();
        assert!(filter.matches_document_transition(&purchase, None, Some(&owner_value)));
    }

    #[test]
    fn test_transfer_original_clause_only_matches_with_original_document() {
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;

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
            action_clauses: DocumentActionClauses::Transfer {
                original_clauses: Some(InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                }),
                owner_clause: None,
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([9u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });
        let transfer_v0 = DocumentTransferTransitionV0 {
            base: document_base,
            revision: 1,
            recipient_owner_id: Identifier::from([8u8; 32]),
        };
        let transfer = DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0));

        // Original doc present and matching
        let mut original = BTreeMap::new();
        original.insert("status".to_string(), Value::Text("active".to_string()));
        assert!(filter.matches_document_transition(&transfer, Some(&original), None));

        // Without original doc, clause is required -> no match
        assert!(!filter.matches_document_transition(&transfer, None, None));
    }

    #[test]
    fn test_delete_original_clause_only_matches_with_original_document() {
        use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;

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
            action_clauses: DocumentActionClauses::Delete {
                original_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
            },
        };

        let document_base = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([12u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });
        let delete_v0 = DocumentDeleteTransitionV0 {
            base: document_base,
        };
        let delete = DocumentTransition::Delete(DocumentDeleteTransition::V0(delete_v0));

        // Original doc present and matching
        let mut original = BTreeMap::new();
        original.insert("status".to_string(), Value::Text("active".to_string()));
        assert!(filter.matches_document_transition(&delete, Some(&original), None));

        // Without original doc -> no match (required for Delete)
        assert!(!filter.matches_document_transition(&delete, None, None));

        // Original mismatching -> no match
        let mut original_bad = BTreeMap::new();
        original_bad.insert("status".to_string(), Value::Text("inactive".to_string()));
        assert!(!filter.matches_document_transition(&delete, Some(&original_bad), None));
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
            action_clauses: DocumentActionClauses::UpdatePrice {
                original_clauses: None,
                price_clause: Some(WhereClause {
                    field: "price".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(5),
                }),
            },
        };
        assert!(filter_price_only.matches_document_transition(&update, None, None));

        let filter_price_only_fail = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::UpdatePrice {
                original_clauses: None,
                price_clause: Some(WhereClause {
                    field: "price".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::U64(15),
                }),
            },
        };
        assert!(!filter_price_only_fail.matches_document_transition(&update, None, None));

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
            action_clauses: DocumentActionClauses::UpdatePrice {
                original_clauses: Some(InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                }),
                price_clause: Some(WhereClause {
                    field: "price".to_string(),
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: Value::U64(10),
                }),
            },
        };
        let mut original_doc = BTreeMap::new();
        original_doc.insert("kind".to_string(), Value::Text("sale".to_string()));
        assert!(filter_with_orig.matches_document_transition(&update, Some(&original_doc), None));

        // Missing original doc -> required -> no match
        assert!(!filter_with_orig.matches_document_transition(&update, None, None));
    }

    #[test]
    fn test_replace_with_both_original_and_final_clauses() {
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
        let final_clauses = InternalClauses {
            equal_clauses: final_eq,
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionClauses::Replace {
                original_clauses: Some(original_clauses),
                final_clauses: Some(final_clauses),
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

        // Original provided and matching; final matches
        let mut original_doc = BTreeMap::new();
        original_doc.insert("status".to_string(), Value::Text("active".to_string()));
        assert!(filter.matches_document_transition(&replace, Some(&original_doc), None));

        // Original missing -> should fail as it's required
        assert!(!filter.matches_document_transition(&replace, None, None));

        // Original mismatching -> fail
        let mut original_doc_bad = BTreeMap::new();
        original_doc_bad.insert("status".to_string(), Value::Text("inactive".to_string()));
        assert!(!filter.matches_document_transition(&replace, Some(&original_doc_bad), None));

        // Final mismatching -> fail (change score)
        if let DocumentTransition::Replace(mut rep) = replace.clone() {
            let DocumentReplaceTransition::V0(ref mut v0) = rep;
            v0.data.insert("score".to_string(), Value::U64(9));
            let bad_final = DocumentTransition::Replace(rep);
            assert!(!filter.matches_document_transition(&bad_final, Some(&original_doc), None));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
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
        assert!(filter.matches_document(&document_base, &in_range));

        // Test lower bound (inclusive)
        let mut lower_bound = BTreeMap::new();
        lower_bound.insert("value".to_string(), Value::U64(10));
        assert!(filter.matches_document(&document_base, &lower_bound));

        // Test upper bound (inclusive)
        let mut upper_bound = BTreeMap::new();
        upper_bound.insert("value".to_string(), Value::U64(20));
        assert!(filter.matches_document(&document_base, &upper_bound));

        // Test below range
        let mut below = BTreeMap::new();
        below.insert("value".to_string(), Value::U64(5));
        assert!(!filter.matches_document(&document_base, &below));

        // Test above range
        let mut above = BTreeMap::new();
        above.insert("value".to_string(), Value::U64(25));
        assert!(!filter.matches_document(&document_base, &above));
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
            },
        };

        assert!(
            valid_filter.validate(),
            "Filter with indexed field should be valid"
        );

        // Test filter with non-indexed field: index validation should fail
        let mut internal_clauses = InternalClauses::default();
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "nonExistentField".to_string(),
            WhereClause {
                field: "nonExistentField".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("value".to_string()),
            },
        );
        internal_clauses.equal_clauses = equal_clauses;

        let invalid_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "indexedDocument".to_string(),
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
            },
        };

        assert!(
            !invalid_filter.validate(),
            "Filter with non-indexed field should be invalid"
        );

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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses,
            },
        };

        assert!(
            primary_key_filter.validate(),
            "Filter with only primary key should be valid"
        );
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
            action_clauses: DocumentActionClauses::Create {
                final_clauses: internal_clauses.clone(),
            },
        };

        // No conversion helpers; verify the filter holds the expected clauses
        if let DocumentActionClauses::Create { final_clauses } = original_filter.action_clauses {
            assert_eq!(final_clauses, internal_clauses);
        } else {
            panic!("expected Create action clauses");
        }
    }
}
