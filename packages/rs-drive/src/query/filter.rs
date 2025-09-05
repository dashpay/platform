//! DriveDocumentQueryFilter and related functionality
//!
//! This module defines the `DriveDocumentQueryFilter` struct, which is used to filter
//! document state transitions based on specified criteria. It includes methods to check
//! if a state transition or document matches the filter, as well as utility functions for
//! evaluating where clauses.
//!
//! The filter is primarily used in the context of clients subscribing to document events
//! in Platform.

use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::platform_value::Value;
use dpp::version::LATEST_PLATFORM_VERSION;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::StateTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use indexmap::IndexMap;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};

#[cfg(any(feature = "server", feature = "verify"))]
/// DriveDocumentQueryFilter struct for filtering document state transitions
#[derive(Debug, PartialEq, Clone)]
pub struct DriveDocumentQueryFilter<'a> {
    /// DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
    /// Internal clauses
    pub internal_clauses: InternalClauses,
}

impl<'a> From<DriveDocumentQueryFilter<'a>> for DriveDocumentQuery<'a> {
    fn from(value: DriveDocumentQueryFilter<'a>) -> Self {
        DriveDocumentQuery {
            contract: value.contract,
            document_type: value.document_type,
            internal_clauses: value.internal_clauses,
            offset: None,
            limit: None,
            order_by: IndexMap::new(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        }
    }
}

impl<'a> From<DriveDocumentQuery<'a>> for DriveDocumentQueryFilter<'a> {
    fn from(value: DriveDocumentQuery<'a>) -> Self {
        DriveDocumentQueryFilter {
            contract: value.contract,
            document_type: value.document_type,
            internal_clauses: value.internal_clauses,
        }
    }
}

impl DriveDocumentQueryFilter<'_> {
    /// Checks if a state transition matches the filter
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_state_transition(&self, state_transition: &StateTransition) -> bool {
        match state_transition {
            StateTransition::Batch(batch) => {
                for transition in batch.transitions_iter() {
                    if let BatchedTransitionRef::Document(document_transition) = transition {
                        if self.matches_document_state_transition(document_transition) {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Checks if a document state transition matches the filter
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document_state_transition(
        &self,
        document_transition: &DocumentTransition,
    ) -> bool {
        match document_transition {
            DocumentTransition::Create(create) => {
                self.matches_document(create.base(), create.data())
            }
            DocumentTransition::Replace(replace) => {
                self.matches_document(replace.base(), replace.data())
            }
            DocumentTransition::Delete(_) => {
                todo!()
            }
            DocumentTransition::Transfer(_) => {
                todo!()
            }
            DocumentTransition::UpdatePrice(_) => {
                todo!()
            }
            DocumentTransition::Purchase(_) => {
                todo!()
            }
        }
    }

    /// Checks if a document (base transition and data) matches the filter
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document(
        &self,
        document_base_transition: &DocumentBaseTransition,
        document_data: &BTreeMap<String, Value>,
    ) -> bool {
        // Check contract ID
        if document_base_transition.data_contract_id() != self.contract.id() {
            return false;
        }

        // Check document type name
        if document_base_transition.document_type_name() != self.document_type.name() {
            return false;
        }

        // Check primary key in clause (for document ID)
        if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause {
            if !self.evaluate_where_clause(
                primary_key_in_clause,
                &Value::Identifier(document_base_transition.id().to_buffer()),
            ) {
                return false;
            }
        }

        // Check primary key equal clause (for document ID)
        if let Some(primary_key_equal_clause) = &self.internal_clauses.primary_key_equal_clause {
            if !self.evaluate_where_clause(
                primary_key_equal_clause,
                &Value::Identifier(document_base_transition.id().to_buffer()),
            ) {
                return false;
            }
        }

        // Check in clause
        if let Some(in_clause) = &self.internal_clauses.in_clause {
            let field_value = document_data.get(&in_clause.field);
            if let Some(value) = field_value {
                if !self.evaluate_where_clause(in_clause, value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check range clause
        if let Some(range_clause) = &self.internal_clauses.range_clause {
            let field_value = document_data.get(&range_clause.field);
            if let Some(value) = field_value {
                if !self.evaluate_where_clause(range_clause, value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check equal clauses
        for (field, equal_clause) in &self.internal_clauses.equal_clauses {
            let field_value = document_data.get(field);
            if let Some(value) = field_value {
                if !self.evaluate_where_clause(equal_clause, value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Helper function to evaluate a where clause against a value
    #[cfg(any(feature = "server", feature = "verify"))]
    fn evaluate_where_clause(&self, clause: &WhereClause, value: &Value) -> bool {
        match &clause.operator {
            WhereOperator::Equal => value == &clause.value,
            WhereOperator::GreaterThan => value > &clause.value,
            WhereOperator::GreaterThanOrEquals => value >= &clause.value,
            WhereOperator::LessThan => value < &clause.value,
            WhereOperator::LessThanOrEquals => value <= &clause.value,
            WhereOperator::In => {
                if let Value::Array(ref array) = clause.value {
                    array.contains(value)
                } else {
                    false
                }
            }
            WhereOperator::Between => {
                if let Value::Array(ref bounds) = clause.value {
                    if bounds.len() == 2 {
                        value >= &bounds[0] && value <= &bounds[1]
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            WhereOperator::BetweenExcludeBounds => {
                if let Value::Array(ref bounds) = clause.value {
                    if bounds.len() == 2 {
                        value > &bounds[0] && value < &bounds[1]
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            WhereOperator::BetweenExcludeLeft => {
                if let Value::Array(ref bounds) = clause.value {
                    if bounds.len() == 2 {
                        value > &bounds[0] && value <= &bounds[1]
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            WhereOperator::BetweenExcludeRight => {
                if let Value::Array(ref bounds) = clause.value {
                    if bounds.len() == 2 {
                        value >= &bounds[0] && value < &bounds[1]
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            WhereOperator::StartsWith => {
                if let (Value::Text(text), Value::Text(prefix)) = (value, &clause.value) {
                    text.starts_with(prefix.as_str())
                } else {
                    false
                }
            }
        }
    }

    /// Validates that the filter's clauses are valid for the document type
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate(&self) -> bool {
        // Convert the filter to a query to reuse find_best_index logic
        let query: DriveDocumentQuery = self.clone().into();
        query.find_best_index(LATEST_PLATFORM_VERSION).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
    use dpp::tests::fixtures::get_data_contract_fixture;

    #[test]
    fn test_matches_document_basic() {
        // Get a test contract from fixtures
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

        // Create a filter with no clauses (should match if contract and type match)
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses::default(),
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
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

        let target_id = Identifier::from([42u8; 32]);

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.primary_key_equal_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(target_id.to_buffer()),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type,
            internal_clauses,
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
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

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
            document_type,
            internal_clauses,
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
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

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
            document_type,
            internal_clauses,
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
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

        // Test GreaterThan
        let mut internal_clauses = InternalClauses::default();
        internal_clauses.range_clause = Some(WhereClause {
            field: "score".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(50),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type,
            internal_clauses,
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
    fn test_matches_document_with_between_operator() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.range_clause = Some(WhereClause {
            field: "value".to_string(),
            operator: WhereOperator::Between,
            value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
        });

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type,
            internal_clauses,
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
        let document_type = contract
            .document_type_for_name("indexedDocument")
            .expect("document type should exist");

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
            document_type,
            internal_clauses,
        };

        assert!(
            valid_filter.validate(),
            "Filter with indexed field should be valid"
        );

        // Test invalid filter with non-indexed field
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
            document_type,
            internal_clauses,
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
            document_type,
            internal_clauses,
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
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("document type should exist");

        let mut internal_clauses = InternalClauses::default();
        internal_clauses.primary_key_equal_clause = Some(WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier([42u8; 32]),
        });

        let original_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type,
            internal_clauses: internal_clauses.clone(),
        };

        // Convert to DriveDocumentQuery
        let query: DriveDocumentQuery = original_filter.clone().into();

        // Check that core fields are preserved
        assert_eq!(query.contract.id(), contract.id());
        assert_eq!(query.document_type.name(), document_type.name());
        assert_eq!(query.internal_clauses, internal_clauses);

        // Check that optional fields are set to defaults
        assert_eq!(query.offset, None);
        assert_eq!(query.limit, None);
        assert!(query.order_by.is_empty());
        assert_eq!(query.start_at, None);
        assert_eq!(query.start_at_included, false);
        assert_eq!(query.block_time_ms, None);

        // Convert back to filter
        let converted_filter: DriveDocumentQueryFilter = query.into();

        // Should preserve the internal clauses
        assert_eq!(
            converted_filter.internal_clauses,
            original_filter.internal_clauses
        );
    }
}
