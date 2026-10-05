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
//! - Delete: evaluates `original_document_clauses` on the original document, or on an
//!   indexOnly delete's values.
//! - A create's and a replace's data, and an indexOnly delete's values, are read as the
//!   platform stores them: with every `generatedFrom` property the transition leaves out
//!   generated from its params (protocol version 14).
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

use std::borrow::Cow;
use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::document::{Document, DocumentV0Getters};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use crate::query::{InternalClauses, QuerySyntaxSimpleValidationResult, ValueClause, WhereClause, WhereOperator};
use crate::error::query::QuerySyntaxError;
use dpp::platform_value::ValueMapHelper;
use dpp::version::PlatformVersion;

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
#[allow(clippy::large_enum_variant)]
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

#[cfg(any(feature = "server", feature = "verify"))]
impl DocumentActionMatchClauses {
    /// Every clause set the action carries, original-document clauses first.
    pub fn clause_sets(&self) -> Vec<&InternalClauses> {
        match self {
            DocumentActionMatchClauses::Create {
                new_document_clauses,
            } => vec![new_document_clauses],
            DocumentActionMatchClauses::Replace {
                original_document_clauses,
                new_document_clauses,
            } => vec![original_document_clauses, new_document_clauses],
            DocumentActionMatchClauses::Delete {
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
            } => vec![original_document_clauses],
        }
    }
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
    ///
    /// A create's and a replace's data, and an indexOnly delete's values, are judged as
    /// the platform stores them at `platform_version`: a `generatedFrom` property the
    /// transition leaves out is generated from its params first.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document_transition(
        &self,
        document_transition: &DocumentTransition,
        batch_owner_value: Option<&Value>, // Only used for Purchase
        platform_version: &PlatformVersion,
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
                    let Some(data) = self.data_as_stored(create.data(), platform_version) else {
                        return TransitionCheckResult::Fail;
                    };
                    if self.evaluate_clauses(
                        new_document_clauses,
                        &id_value,
                        &data,
                        Some(platform_version),
                    ) {
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
                        let Some(data) = self.data_as_stored(replace.data(), platform_version)
                        else {
                            return TransitionCheckResult::Fail;
                        };
                        self.evaluate_clauses(
                            new_document_clauses,
                            &id_value,
                            &data,
                            Some(platform_version),
                        )
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
                            None,
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
                            None,
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
                            None,
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
                            None,
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
                            None,
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
            DocumentTransition::IndexOnlyDelete(index_only_delete) => {
                if let DocumentActionMatchClauses::Delete {
                    original_document_clauses,
                } = &self.action_clauses
                {
                    // An indexOnly document has no stored row to fetch:
                    // the transition carries the document's full values,
                    // so the "original document" clauses evaluate
                    // directly against them and no original is ever
                    // needed.
                    let Some(values) =
                        self.data_as_stored(index_only_delete.data(), platform_version)
                    else {
                        return TransitionCheckResult::Fail;
                    };
                    if self.evaluate_clauses(
                        original_document_clauses,
                        &id_value,
                        &values,
                        Some(platform_version),
                    ) {
                        TransitionCheckResult::Pass
                    } else {
                        TransitionCheckResult::Fail
                    }
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
                // A stored document's properties already carry the types its schema gives them.
                self.evaluate_clauses(
                    original_document_clauses,
                    &id_value,
                    original_document.properties(),
                    None,
                )
            }
            _ => false,
        }
    }

    /// A transition's `data` as the platform stores it, every `generatedFrom` property it
    /// leaves out generated ([`DocumentTypeBasicMethods::data_as_stored`]). `None`, which
    /// fails the match, for a document type the contract lacks (`validate` refuses the
    /// filter then) or a version table this build does not know.
    #[cfg(any(feature = "server", feature = "verify"))]
    fn data_as_stored<'d>(
        &self,
        data: &'d BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Option<Cow<'d, BTreeMap<String, Value>>> {
        let document_type = self
            .contract
            .document_type_for_name(&self.document_type_name)
            .ok()?;
        document_type.data_as_stored(data, platform_version).ok()
    }

    /// Single clause evaluator used by both transition and original-document paths.
    ///
    /// With `platform_version`, each value a clause reads is first brought to the form the
    /// document type stores ([`canonical_value_for_key`]): a transition keeps whichever
    /// encoding its submitter chose (an identifier as bytes, a `u8` property as `U64`), while
    /// clause values were canonicalized once by
    /// [`Self::canonicalize_clause_values`]. A value that does not encode as its property
    /// fails the clause. `None` is for data that is already canonical, such as a stored
    /// document's properties.
    #[cfg(any(feature = "server", feature = "verify"))]
    fn evaluate_clauses(
        &self,
        clauses: &InternalClauses,
        document_id_value: &Value,
        document_data: &BTreeMap<String, Value>,
        platform_version: Option<&PlatformVersion>,
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

        let document_type = match platform_version {
            Some(_) => match self
                .contract
                .document_type_optional_for_name(&self.document_type_name)
            {
                Some(document_type) => Some(document_type),
                None => return false,
            },
            None => None,
        };
        let field_matches = |clause: &WhereClause| {
            let Some(value) = get_value_by_path(document_data, &clause.field) else {
                return false;
            };
            match (document_type, platform_version) {
                (Some(document_type), Some(platform_version)) => {
                    match canonical_value_for_key(
                        document_type,
                        &clause.field,
                        value,
                        platform_version,
                    ) {
                        Some(canonical) => clause.matches_value(&canonical),
                        None => false,
                    }
                }
                _ => clause.matches_value(value),
            }
        };

        clauses.in_clauses.iter().all(&field_matches)
            && clauses.range_clause.iter().all(&field_matches)
            && clauses.equal_clauses.values().all(&field_matches)
    }

    /// Bring every clause value to the form the document type stores its field in, so
    /// [`Self::matches_document_transition`] compares like with like: an identifier given as
    /// bytes or base58 text becomes `Value::Identifier`, an integer takes its property's
    /// width, `IN` and `BETWEEN*` operands are converted element-wise. Clauses on the
    /// recipient or buyer (`owner_clause`) are read as identifiers and `price_clause` as
    /// `U64`. Call it before [`Self::validate`], which checks the canonical types.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn canonicalize_clause_values(
        &mut self,
        platform_version: &PlatformVersion,
    ) -> Result<(), QuerySyntaxError> {
        let contract = self.contract;
        let document_type = contract
            .document_type_optional_for_name(&self.document_type_name)
            .ok_or(QuerySyntaxError::DocumentTypeNotFound(
                "unknown document type",
            ))?;
        let (clause_sets, scalar_clause): (Vec<&mut InternalClauses>, _) =
            match &mut self.action_clauses {
                DocumentActionMatchClauses::Create {
                    new_document_clauses,
                } => (vec![new_document_clauses], None),
                DocumentActionMatchClauses::Replace {
                    original_document_clauses,
                    new_document_clauses,
                } => (vec![original_document_clauses, new_document_clauses], None),
                DocumentActionMatchClauses::Delete {
                    original_document_clauses,
                } => (vec![original_document_clauses], None),
                DocumentActionMatchClauses::Transfer {
                    original_document_clauses,
                    owner_clause,
                }
                | DocumentActionMatchClauses::Purchase {
                    original_document_clauses,
                    owner_clause,
                } => (
                    vec![original_document_clauses],
                    owner_clause.as_mut().map(|clause| ("$ownerId", clause)),
                ),
                DocumentActionMatchClauses::UpdatePrice {
                    original_document_clauses,
                    price_clause,
                } => (
                    vec![original_document_clauses],
                    price_clause.as_mut().map(|clause| ("$price", clause)),
                ),
            };
        for clauses in clause_sets {
            let InternalClauses {
                primary_key_in_clause,
                primary_key_equal_clause,
                in_clauses,
                range_clause,
                equal_clauses,
            } = clauses;
            for clause in primary_key_in_clause
                .iter_mut()
                .chain(primary_key_equal_clause.iter_mut())
                .chain(in_clauses.iter_mut())
                .chain(range_clause.iter_mut())
                .chain(equal_clauses.values_mut())
            {
                // A `u8` field's IN candidates may come packed as bytes, already canonical.
                if clause.operator == WhereOperator::In
                    && matches!(clause.value, Value::Bytes(_))
                    && matches!(
                        document_type
                            .flattened_properties()
                            .get(&clause.field)
                            .map(|property| &property.property_type),
                        Some(DocumentPropertyType::U8)
                    )
                {
                    continue;
                }
                clause.value = canonical_operand(clause.operator, &clause.value, |value| {
                    canonical_value_for_key(document_type, &clause.field, value, platform_version)
                })?;
            }
        }
        if let Some((key, clause)) = scalar_clause {
            clause.value = canonical_operand(clause.operator, &clause.value, |value| {
                if key == "$price" {
                    value.to_integer::<u64>().ok().map(Value::U64)
                } else {
                    canonical_value_for_key(document_type, key, value, platform_version)
                }
            })?;
        }
        Ok(())
    }

    /// Validate the filter structure and clauses.
    ///
    /// In addition to these validations, the subscription host should check the contract's existence.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate(&self) -> QuerySyntaxSimpleValidationResult {
        // Ensure the document type exists
        let Some(document_type) = self
            .contract
            .document_type_optional_for_name(&self.document_type_name)
        else {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::DocumentTypeNotFound("unknown document type"),
            );
        };

        // Document data carries no `$`-prefixed system fields, so a clause on one (other than
        // the primary-key `$id` clauses) could never match.
        if self
            .action_clauses
            .clause_sets()
            .into_iter()
            .any(InternalClauses::names_system_field)
        {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "filter clauses may not name a `$` system field other than `$id`",
                ),
            );
        }

        match &self.action_clauses {
            DocumentActionMatchClauses::Create {
                new_document_clauses,
            } => new_document_clauses.validate_against_schema(document_type),
            DocumentActionMatchClauses::Replace {
                original_document_clauses,
                new_document_clauses,
            } => {
                if !original_document_clauses.is_empty() {
                    let result = original_document_clauses.validate_against_schema(document_type);
                    if result.is_err() {
                        return result;
                    }
                }
                if !new_document_clauses.is_empty() {
                    new_document_clauses.validate_against_schema(document_type)
                } else {
                    QuerySyntaxSimpleValidationResult::new()
                }
            }
            DocumentActionMatchClauses::Delete {
                original_document_clauses,
            } => original_document_clauses.validate_against_schema(document_type),
            DocumentActionMatchClauses::Transfer {
                original_document_clauses,
                owner_clause,
            } => {
                if !original_document_clauses.is_empty() {
                    let result = original_document_clauses.validate_against_schema(document_type);
                    if result.is_err() {
                        return result;
                    }
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
                    if ok {
                        QuerySyntaxSimpleValidationResult::new()
                    } else {
                        QuerySyntaxSimpleValidationResult::new_with_error(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid owner clause"),
                        )
                    }
                } else {
                    QuerySyntaxSimpleValidationResult::new()
                }
            }
            DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses,
                price_clause,
            } => {
                if !original_document_clauses.is_empty() {
                    let result = original_document_clauses.validate_against_schema(document_type);
                    if result.is_err() {
                        return result;
                    }
                }
                if let Some(price) = price_clause {
                    let ok = match price.operator {
                        WhereOperator::Equal
                        | WhereOperator::GreaterThan
                        | WhereOperator::GreaterThanOrEquals
                        | WhereOperator::LessThan
                        | WhereOperator::LessThanOrEquals => {
                            price.value.is_integer_can_fit_in_64_bits()
                        }
                        WhereOperator::Between
                        | WhereOperator::BetweenExcludeBounds
                        | WhereOperator::BetweenExcludeLeft
                        | WhereOperator::BetweenExcludeRight => match &price.value {
                            Value::Array(arr) => {
                                arr.len() == 2
                                    && arr.iter().all(|v| v.is_integer_can_fit_in_64_bits())
                                    && arr[0] < arr[1]
                            }
                            _ => false,
                        },
                        WhereOperator::In => match &price.value {
                            Value::Array(arr) => {
                                arr.iter().all(|v| v.is_integer_can_fit_in_64_bits())
                            }
                            _ => false,
                        },
                        WhereOperator::StartsWith => false,
                    };
                    if ok {
                        QuerySyntaxSimpleValidationResult::new()
                    } else {
                        QuerySyntaxSimpleValidationResult::new_with_error(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid price clause"),
                        )
                    }
                } else {
                    QuerySyntaxSimpleValidationResult::new()
                }
            }
            DocumentActionMatchClauses::Purchase {
                original_document_clauses,
                owner_clause,
            } => {
                if !original_document_clauses.is_empty() {
                    let result = original_document_clauses.validate_against_schema(document_type);
                    if result.is_err() {
                        return result;
                    }
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
                    if ok {
                        QuerySyntaxSimpleValidationResult::new()
                    } else {
                        QuerySyntaxSimpleValidationResult::new_with_error(
                            QuerySyntaxError::InvalidWhereClauseComponents("invalid owner clause"),
                        )
                    }
                } else {
                    QuerySyntaxSimpleValidationResult::new()
                }
            }
        }
    }
}

/// `value` as the document type stores `key`: encoded the way its index keys are, then
/// decoded back, so every accepted encoding of one value yields the same `Value`. `None` when
/// the value does not encode as the property (wrong type, wrong length) or the type has no
/// such property.
///
/// Schema properties go through their property type's codec directly rather than
/// `serialize_value_for_key`, which refuses values longer than an index key may be: a filter
/// may read a field no index covers, and a long value must still compare.
#[cfg(any(feature = "server", feature = "verify"))]
fn canonical_value_for_key(
    document_type: DocumentTypeRef,
    key: &str,
    value: &Value,
    platform_version: &PlatformVersion,
) -> Option<Value> {
    match document_type
        .flattened_properties()
        .get(key)
        .map(|property| &property.property_type)
    {
        Some(property_type) => canonical_property_value(property_type, value),
        // `$id` and the recipient or buyer (read as `$ownerId`) are the only other fields a
        // filter reads. Anything else, such as a derived index property, is not in a
        // transition's data and is refused before its codec runs.
        None if SYSTEM_IDENTIFIER_FIELDS.contains(&key) => {
            if exceeds_identifier_text(value) {
                return None;
            }
            let serialized = document_type
                .serialize_value_for_key(key, value, platform_version)
                .ok()?;
            document_type
                .deserialize_value_for_key(key, &serialized, platform_version)
                .ok()
        }
        None => None,
    }
}

/// `value` as a property of `property_type` is stored.
#[cfg(any(feature = "server", feature = "verify"))]
fn canonical_property_value(property_type: &DocumentPropertyType, value: &Value) -> Option<Value> {
    match property_type {
        // Text is already canonical, and the index-key form would conflate `""` with `"\0"`.
        DocumentPropertyType::String(_) => matches!(value, Value::Text(_)).then(|| value.clone()),
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
            if exceeds_identifier_text(value) =>
        {
            None
        }
        _ => {
            let encoded = property_type.encode_value_for_tree_keys(value).ok()?;
            // The index-key form of an empty byte array is empty, which decodes as null.
            if encoded.is_empty() && !value.is_null() {
                return match property_type {
                    DocumentPropertyType::ByteArray(_) => Some(Value::Bytes(vec![])),
                    _ => None,
                };
            }
            property_type.decode_value_for_tree_keys(&encoded).ok()
        }
    }
}

/// Base58 decoding takes time quadratic in its input, and a filter's operands come from
/// unauthenticated requests: identifier text longer than any 32-byte identifier is refused
/// before it is decoded.
#[cfg(any(feature = "server", feature = "verify"))]
fn exceeds_identifier_text(value: &Value) -> bool {
    matches!(value, Value::Text(text) if text.len() > MAX_BASE58_IDENTIFIER_LEN)
}

/// The system fields a filter may read, all identifiers.
#[cfg(any(feature = "server", feature = "verify"))]
const SYSTEM_IDENTIFIER_FIELDS: [&str; 2] = ["$id", "$ownerId"];

/// The longest base58 text of a 32-byte identifier.
#[cfg(any(feature = "server", feature = "verify"))]
const MAX_BASE58_IDENTIFIER_LEN: usize = 44;

/// Canonicalize a clause operand with `canonical`: element-wise for the list operands of
/// `IN` and `BETWEEN*`, directly otherwise.
#[cfg(any(feature = "server", feature = "verify"))]
fn canonical_operand(
    operator: WhereOperator,
    operand: &Value,
    canonical: impl Fn(&Value) -> Option<Value>,
) -> Result<Value, QuerySyntaxError> {
    let convert = |value: &Value| {
        canonical(value).ok_or_else(|| {
            QuerySyntaxError::InvalidFormatWhereClause(format!(
                "where clause value {value} does not match the type of its field"
            ))
        })
    };
    match (operator, operand) {
        (
            WhereOperator::In
            | WhereOperator::Between
            | WhereOperator::BetweenExcludeBounds
            | WhereOperator::BetweenExcludeLeft
            | WhereOperator::BetweenExcludeRight,
            Value::Array(values),
        ) => Ok(Value::Array(
            values.iter().map(convert).collect::<Result<_, _>>()?,
        )),
        _ => convert(operand),
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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &document_data, None));
    }

    #[test]
    fn test_matches_document_with_primary_key_equal() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let target_id = Identifier::from([42u8; 32]);

        let internal_clauses = InternalClauses {
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: target_id.into(),
            }),
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &document_data, None));

        // Test with different ID
        let non_matching_doc = DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
            id: Identifier::from([99u8; 32]),
            document_type_name: "niceDocument".to_string(),
            data_contract_id: contract.id(),
            identity_contract_nonce: 0,
            token_payment_info: None,
        });

        let non_id_value: Value = non_matching_doc.id().into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &non_id_value, &document_data, None));
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

        let internal_clauses = InternalClauses {
            equal_clauses,
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &matching_data, None));

        // Test with non-matching data
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("name".to_string(), Value::Text("different".to_string()));

        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &non_matching_data, None));

        // Test with missing field
        let empty_data = BTreeMap::new();
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &empty_data, None));
    }

    #[test]
    fn test_matches_document_with_in_operator() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let allowed_values = vec![
            Value::Text("active".to_string()),
            Value::Text("pending".to_string()),
        ];

        let internal_clauses = InternalClauses {
            in_clauses: vec![WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(allowed_values),
            }],
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &matching_data, None));

        // Test with value not in list
        let mut non_matching_data = BTreeMap::new();
        non_matching_data.insert("status".to_string(), Value::Text("completed".to_string()));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &non_matching_data, None));
    }

    #[test]
    fn test_matches_document_with_range_operators() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Test GreaterThan
        let internal_clauses = InternalClauses {
            range_clause: Some(WhereClause {
                field: "score".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::U64(50),
            }),
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &greater_data, None));

        // Test with value equal to threshold (should fail for GreaterThan)
        let mut equal_data = BTreeMap::new();
        equal_data.insert("score".to_string(), Value::U64(50));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &equal_data, None));

        // Test with value less than threshold
        let mut less_data = BTreeMap::new();
        less_data.insert("score".to_string(), Value::U64(25));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &less_data, None));
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

        let internal_clauses = InternalClauses {
            equal_clauses,
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &data, None));
    }

    #[test]
    fn test_validate_optional_actions_allow_empty_clauses() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Replace with none/none -> allowed
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Replace {
                original_document_clauses: InternalClauses::default(),
                new_document_clauses: InternalClauses::default(),
            },
        };
        assert!(filter.validate().is_valid());

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
        assert!(filter.validate().is_valid());

        // Transfer with none/none -> allowed
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: None,
            },
        };
        assert!(filter.validate().is_valid());

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
        assert!(filter.validate().is_valid());

        // UpdatePrice with none/none -> allowed
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: None,
            },
        };
        assert!(filter.validate().is_valid());

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
        assert!(filter.validate().is_valid());

        // Purchase with none/none -> allowed
        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: None,
            },
        };
        assert!(filter.validate().is_valid());

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
        assert!(filter.validate().is_valid());
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
            revision: 1_u64,
            recipient_owner_id: new_owner,
        };
        let transfer = DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0));

        // First check should pass without needing original
        assert_eq!(
            filter.matches_document_transition(&transfer, None, PlatformVersion::latest()),
            TransitionCheckResult::Pass
        );

        // Mismatch owner
        let other_owner = Identifier::from([6u8; 32]);
        let transfer_v0_mismatch = DocumentTransferTransitionV0 {
            base: document_base,
            revision: 1_u64,
            recipient_owner_id: other_owner,
        };
        let transfer_mismatch =
            DocumentTransition::Transfer(DocumentTransferTransition::V0(transfer_v0_mismatch));
        assert_eq!(
            filter.matches_document_transition(&transfer_mismatch, None, PlatformVersion::latest()),
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
            revision: 1_u64,
            price: 10 as Credits,
        };
        let purchase = DocumentTransition::Purchase(DocumentPurchaseTransition::V0(purchase_v0));

        // Without batch owner context, should fail (owner clause requires it)
        assert_eq!(
            filter.matches_document_transition(&purchase, None, PlatformVersion::latest()),
            TransitionCheckResult::Fail
        );
        // With batch owner context, should pass
        let owner_value = Value::Identifier(purchaser.to_buffer());
        assert_eq!(
            filter.matches_document_transition(
                &purchase,
                Some(&owner_value),
                PlatformVersion::latest()
            ),
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
            filter_price_only.matches_document_transition(&update, None, PlatformVersion::latest()),
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
            filter_price_only_fail.matches_document_transition(
                &update,
                None,
                PlatformVersion::latest()
            ),
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
            filter_with_orig.matches_document_transition(&update, None, PlatformVersion::latest()),
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

        // Original must have status=active; new must have name=example
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
            "name".to_string(),
            WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("example".to_string()),
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
                new_document_clauses,
            },
        };

        // Build Replace transition with new data
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("example".to_string()));
        let replace_v0 = DocumentReplaceTransitionV0 {
            base,
            revision: 1,
            data,
        };
        let replace = DocumentTransition::Replace(DocumentReplaceTransition::V0(replace_v0));

        // Original provided and matching; final matches (requires original)
        let mut original_doc = BTreeMap::new();
        original_doc.insert("status".to_string(), Value::Text("active".to_string()));
        assert_eq!(
            filter.matches_document_transition(&replace, None, PlatformVersion::latest()),
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
            v0.data
                .insert("name".to_string(), Value::Text("other".to_string()));
            let bad_final = DocumentTransition::Replace(rep);
            assert_eq!(
                filter.matches_document_transition(&bad_final, None, PlatformVersion::latest()),
                TransitionCheckResult::Fail
            );
        }
    }

    /// A `handle` type whose `normalizedLabel` is generated from `label`, and an
    /// indexOnly `entry` type whose `normalizedName` is generated from `name`
    /// (protocol version 14).
    fn generated_from_contract() -> DataContract {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        let generated = |source: &str| {
            platform_value!({
                "function": "sys.stringTransformations.homographSafeASCII",
                "params": [source]
            })
        };
        let documents = platform_value!({
            "handle": {
                "type": "object",
                "documentsMutable": true,
                "properties": {
                    "label": { "type": "string", "maxLength": 32, "position": 0 },
                    "normalizedLabel": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 1,
                        "generatedFrom": generated("label")
                    }
                },
                "additionalProperties": false
            },
            "entry": {
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "indices": [
                    {
                        "name": "byNormalizedName",
                        "properties": [{ "normalizedName": "asc" }, { "name": "asc" }],
                        "terminal": "$ownerId"
                    }
                ],
                "properties": {
                    "name": { "type": "string", "maxLength": 32, "position": 0 },
                    "normalizedName": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 1,
                        "generatedFrom": generated("name")
                    }
                },
                "required": ["name", "normalizedName"],
                "additionalProperties": false
            }
        });
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::from([1u8; 32]), 1, documents, None, None)
            .expect("the contract parses")
            .data_contract_owned()
    }

    /// A clause on a `generatedFrom` property judges a create, a replace and an
    /// indexOnly delete that leave the property out as the platform stores them,
    /// with the value generated from its param.
    #[test]
    fn test_matches_a_generated_property_the_transition_leaves_out() {
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::DocumentIndexOnlyDeleteTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::v0::DocumentReplaceTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransition;

        let contract = generated_from_contract();
        let platform_version = PlatformVersion::latest();
        let equal = |field: &str, value: &str| InternalClauses {
            equal_clauses: BTreeMap::from([(
                field.to_string(),
                WhereClause {
                    field: field.to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(value.to_string()),
                },
            )]),
            ..Default::default()
        };
        let base = |document_type_name: &str| {
            DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                id: Identifier::from([3u8; 32]),
                document_type_name: document_type_name.to_string(),
                data_contract_id: contract.id(),
                identity_contract_nonce: 0,
                token_payment_info: None,
            })
        };
        let data = |key: &str, value: &str| {
            BTreeMap::from([(key.to_string(), Value::Text(value.to_string()))])
        };

        let create =
            DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: base("handle"),
                entropy: [0u8; 32],
                data: data("label", "Bob"),
                prefunded_voting_balance: None,
            }));
        let replace = DocumentTransition::Replace(DocumentReplaceTransition::V0(
            DocumentReplaceTransitionV0 {
                base: base("handle"),
                revision: 2,
                data: data("label", "Bob"),
            },
        ));
        let index_only_delete = DocumentTransition::IndexOnlyDelete(
            DocumentIndexOnlyDeleteTransition::V0(DocumentIndexOnlyDeleteTransitionV0 {
                base: base("entry"),
                data: data("name", "Bob"),
            }),
        );

        for (normalized, expected) in [
            ("b0b", TransitionCheckResult::Pass),
            ("bob", TransitionCheckResult::Fail),
        ] {
            let create_filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "handle".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: equal("normalizedLabel", normalized),
                },
            };
            assert_eq!(
                create_filter.matches_document_transition(&create, None, platform_version),
                expected,
                "create, normalizedLabel == {normalized:?}"
            );

            let replace_filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "handle".to_string(),
                action_clauses: DocumentActionMatchClauses::Replace {
                    original_document_clauses: InternalClauses::default(),
                    new_document_clauses: equal("normalizedLabel", normalized),
                },
            };
            assert_eq!(
                replace_filter.matches_document_transition(&replace, None, platform_version),
                expected,
                "replace, normalizedLabel == {normalized:?}"
            );

            let delete_filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "entry".to_string(),
                action_clauses: DocumentActionMatchClauses::Delete {
                    original_document_clauses: equal("normalizedName", normalized),
                },
            };
            assert_eq!(
                delete_filter.matches_document_transition(
                    &index_only_delete,
                    None,
                    platform_version
                ),
                expected,
                "indexOnly delete, normalizedName == {normalized:?}"
            );
        }
    }

    #[test]
    fn test_matches_document_with_between_operator() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let internal_clauses = InternalClauses {
            range_clause: Some(WhereClause {
                field: "value".to_string(),
                operator: WhereOperator::Between,
                value: Value::Array(vec![Value::U64(10), Value::U64(20)]),
            }),
            ..Default::default()
        };

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
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &in_range, None));

        // Test lower bound (inclusive)
        let mut lower_bound = BTreeMap::new();
        lower_bound.insert("value".to_string(), Value::U64(10));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &lower_bound, None));

        // Test upper bound (inclusive)
        let mut upper_bound = BTreeMap::new();
        upper_bound.insert("value".to_string(), Value::U64(20));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &upper_bound, None));

        // Test below range
        let mut below = BTreeMap::new();
        below.insert("value".to_string(), Value::U64(5));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &below, None));

        // Test above range
        let mut above = BTreeMap::new();
        above.insert("value".to_string(), Value::U64(25));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &above, None));
    }

    #[test]
    fn test_validate_filter() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Test valid filter with indexed field
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "firstName".to_string(),
            WhereClause {
                field: "firstName".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("Alice".to_string()),
            },
        );
        let internal_clauses = InternalClauses {
            equal_clauses,
            ..Default::default()
        };

        let valid_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "indexedDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            valid_filter.validate().is_valid(),
            "Filter with indexed field should be valid"
        );

        // Test filter with non-indexed field: structural validation should pass
        // (indexes are not considered by subscription filters).
        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "name".to_string(),
            WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("value".to_string()),
            },
        );
        let internal_clauses = InternalClauses {
            equal_clauses,
            ..Default::default()
        };

        let invalid_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            invalid_filter.validate().is_valid(),
            "Structural validate should ignore indexes"
        );
        // Index-aware validation removed; structural validation suffices for subscriptions.

        // Test valid filter with only primary key
        let internal_clauses = InternalClauses {
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier([42u8; 32]),
            }),
            ..Default::default()
        };

        let primary_key_filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "indexedDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses,
            },
        };

        assert!(
            primary_key_filter.validate().is_valid(),
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

        // Price Between variants must reject equal bounds
        for operator in [
            WhereOperator::Between,
            WhereOperator::BetweenExcludeBounds,
            WhereOperator::BetweenExcludeLeft,
            WhereOperator::BetweenExcludeRight,
        ] {
            let filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "niceDocument".to_string(),
                action_clauses: DocumentActionMatchClauses::UpdatePrice {
                    original_document_clauses: InternalClauses::default(),
                    price_clause: Some(ValueClause {
                        operator,
                        value: Value::Array(vec![Value::U64(10), Value::U64(10)]),
                    }),
                },
            };
            assert!(
                filter.validate().is_err(),
                "{operator:?} should reject equal price bounds"
            );
        }
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

        let internal_clauses = InternalClauses {
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier([42u8; 32]),
            }),
            ..Default::default()
        };

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

    // ---- validate: unknown document type ----

    #[test]
    fn validate_rejects_unknown_document_type() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "doesNotExist".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: InternalClauses::default(),
            },
        };
        let result = filter.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.first_error(),
            Some(QuerySyntaxError::DocumentTypeNotFound(_))
        ));
    }

    // ---- validate: owner clause with In operator ----

    #[test]
    fn validate_transfer_owner_clause_in_with_identifiers_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::Identifier([1u8; 32]),
                        Value::Identifier([2u8; 32]),
                    ]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_transfer_owner_clause_in_with_non_identifiers_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Array(vec![Value::Text("not-an-id".to_string())]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_transfer_owner_clause_greater_than_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThan,
                    value: Value::Identifier([1u8; 32]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    // ---- validate: purchase owner clause ----

    #[test]
    fn validate_purchase_owner_clause_in_with_identifiers_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::Identifier([3u8; 32]),
                        Value::Identifier([4u8; 32]),
                    ]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_purchase_owner_clause_non_identifier_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::Equal,
                    value: Value::U64(42),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_purchase_owner_clause_in_with_non_array_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Identifier([1u8; 32]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    // ---- validate: price clause coverage ----

    #[test]
    fn validate_price_clause_starts_with_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::StartsWith,
                    value: Value::Text("1".to_string()),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_price_clause_in_with_integers_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Array(vec![Value::U64(10), Value::U64(20), Value::U64(30)]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_in_with_non_integer_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Array(vec![Value::Text("not_int".to_string())]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_price_clause_in_with_non_array_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::U64(10),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_price_clause_between_with_valid_integers_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::Between,
                    value: Value::Array(vec![Value::U64(10), Value::U64(100)]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_between_with_descending_bounds_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::Between,
                    value: Value::Array(vec![Value::U64(100), Value::U64(10)]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_price_clause_between_with_non_array_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::Between,
                    value: Value::U64(50),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    #[test]
    fn validate_price_clause_less_than_with_integer_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::LessThan,
                    value: Value::U64(1000),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_less_than_or_equals_with_integer_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::LessThanOrEquals,
                    value: Value::U64(500),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_greater_than_or_equals_with_integer_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::GreaterThanOrEquals,
                    value: Value::U64(50),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    // ---- validate: delete action ----

    #[test]
    fn validate_delete_with_empty_clauses_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Delete {
                original_document_clauses: InternalClauses::default(),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_delete_with_primary_key_clause_is_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Delete {
                original_document_clauses: InternalClauses {
                    primary_key_equal_clause: Some(WhereClause {
                        field: "$id".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Identifier([99u8; 32]),
                    }),
                    ..Default::default()
                },
            },
        };
        assert!(filter.validate().is_valid());
    }

    // ---- matches_original_document: Create action returns false ----

    #[test]
    fn matches_original_document_returns_false_for_create_action() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: InternalClauses::default(),
            },
        };

        let doc = Document::V0(DocumentV0 {
            id: Identifier::from([1u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: BTreeMap::new(),
            ..Default::default()
        });
        // Create has no original document path
        assert!(!filter.matches_original_document(&doc));
    }

    // ---- evaluate_clauses: primary_key_in_clause ----

    #[test]
    fn evaluate_clauses_primary_key_in_clause_matches() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let target_id_1 = Identifier::from([10u8; 32]);
        let target_id_2 = Identifier::from([20u8; 32]);

        let internal_clauses = InternalClauses {
            primary_key_in_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![target_id_1.into(), target_id_2.into()]),
            }),
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        // Matching ID
        let id_value: Value = target_id_1.into();
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &BTreeMap::new(), None));

        // Non-matching ID
        let other_id: Value = Identifier::from([99u8; 32]).into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &other_id, &BTreeMap::new(), None));
    }

    // ---- evaluate_clauses: combined primary_key and field clauses ----

    #[test]
    fn evaluate_clauses_primary_key_plus_equal_clause() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let target_id = Identifier::from([50u8; 32]);

        let mut equal_clauses = BTreeMap::new();
        equal_clauses.insert(
            "name".to_string(),
            WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("test".to_string()),
            },
        );

        let internal_clauses = InternalClauses {
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: target_id.into(),
            }),
            equal_clauses,
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        // Both match
        let id_value: Value = target_id.into();
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("test".to_string()));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &data, None));

        // ID matches but field doesn't
        let mut bad_data = BTreeMap::new();
        bad_data.insert("name".to_string(), Value::Text("other".to_string()));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &bad_data, None));

        // Field matches but ID doesn't
        let wrong_id: Value = Identifier::from([99u8; 32]).into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &wrong_id, &data, None));
    }

    // ---- get_value_by_path ----

    #[test]
    fn get_value_by_path_simple_key() {
        let mut root = BTreeMap::new();
        root.insert("name".to_string(), Value::Text("alice".to_string()));
        let result = get_value_by_path(&root, "name");
        assert_eq!(result, Some(&Value::Text("alice".to_string())));
    }

    #[test]
    fn get_value_by_path_missing_key_returns_none() {
        let root = BTreeMap::new();
        let result = get_value_by_path(&root, "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn get_value_by_path_empty_path_returns_none() {
        let mut root = BTreeMap::new();
        root.insert("x".to_string(), Value::I64(1));
        let result = get_value_by_path(&root, "");
        assert!(result.is_none());
    }

    #[test]
    fn get_value_by_path_nested_map() {
        let nested = vec![(
            Value::Text("level2".to_string()),
            Value::Text("deep_value".to_string()),
        )];
        let mut root = BTreeMap::new();
        root.insert("level1".to_string(), Value::Map(nested));

        let result = get_value_by_path(&root, "level1.level2");
        assert_eq!(result, Some(&Value::Text("deep_value".to_string())));
    }

    #[test]
    fn get_value_by_path_intermediate_non_map_returns_none() {
        let mut root = BTreeMap::new();
        root.insert("scalar".to_string(), Value::I64(42));

        // Trying to traverse through a scalar value
        let result = get_value_by_path(&root, "scalar.child");
        assert!(result.is_none());
    }

    #[test]
    fn get_value_by_path_deeply_nested() {
        let level3 = vec![(Value::Text("val".to_string()), Value::U64(999))];
        let level2 = vec![(Value::Text("c".to_string()), Value::Map(level3))];
        let mut root = BTreeMap::new();
        root.insert("a".to_string(), Value::Map(level2));

        let result = get_value_by_path(&root, "a.c.val");
        assert_eq!(result, Some(&Value::U64(999)));
    }

    #[test]
    fn get_value_by_path_missing_intermediate_key_returns_none() {
        let nested = vec![(Value::Text("exists".to_string()), Value::I64(1))];
        let mut root = BTreeMap::new();
        root.insert("a".to_string(), Value::Map(nested));

        let result = get_value_by_path(&root, "a.not_here.val");
        assert!(result.is_none());
    }

    // ---- evaluate_clauses: range clause with missing field ----

    #[test]
    fn evaluate_clauses_range_clause_missing_field_returns_false() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let internal_clauses = InternalClauses {
            range_clause: Some(WhereClause {
                field: "nonexistent".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::U64(0),
            }),
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let id_value: Value = Identifier::from([1u8; 32]).into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &BTreeMap::new(), None));
    }

    // ---- evaluate_clauses: in clause with missing field ----

    #[test]
    fn evaluate_clauses_in_clause_missing_field_returns_false() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let internal_clauses = InternalClauses {
            in_clauses: vec![WhereClause {
                field: "nonexistent".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![Value::I64(1)]),
            }],
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let id_value: Value = Identifier::from([1u8; 32]).into();
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &BTreeMap::new(), None));
    }

    // ---- matches_original_document: UpdatePrice with original clauses ----

    #[test]
    fn matches_original_document_update_price_evaluates_original_clauses() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut eq = BTreeMap::new();
        eq.insert(
            "kind".to_string(),
            WhereClause {
                field: "kind".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("premium".to_string()),
            },
        );

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                price_clause: None,
            },
        };

        // Matching original
        let mut props = BTreeMap::new();
        props.insert("kind".to_string(), Value::Text("premium".to_string()));
        let doc = Document::V0(DocumentV0 {
            id: Identifier::from([15u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: props,
            ..Default::default()
        });
        assert!(filter.matches_original_document(&doc));

        // Non-matching original
        let mut bad_props = BTreeMap::new();
        bad_props.insert("kind".to_string(), Value::Text("basic".to_string()));
        let bad_doc = Document::V0(DocumentV0 {
            id: Identifier::from([15u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: bad_props,
            ..Default::default()
        });
        assert!(!filter.matches_original_document(&bad_doc));
    }

    // ---- matches_original_document: Purchase with original clauses ----

    #[test]
    fn matches_original_document_purchase_evaluates_original_clauses() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut eq = BTreeMap::new();
        eq.insert(
            "status".to_string(),
            WhereClause {
                field: "status".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("for_sale".to_string()),
            },
        );

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                owner_clause: None,
            },
        };

        let mut props = BTreeMap::new();
        props.insert("status".to_string(), Value::Text("for_sale".to_string()));
        let doc = Document::V0(DocumentV0 {
            id: Identifier::from([20u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties: props,
            ..Default::default()
        });
        assert!(filter.matches_original_document(&doc));
    }

    // ---- validate: Replace with invalid original clauses ----

    #[test]
    fn validate_replace_with_invalid_original_clauses_fails() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        // Put an invalid field name in original clauses
        let mut eq = BTreeMap::new();
        eq.insert(
            "nonexistentField".to_string(),
            WhereClause {
                field: "nonexistentField".to_string(),
                operator: WhereOperator::Equal,
                value: Value::I64(1),
            },
        );

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Replace {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                new_document_clauses: InternalClauses::default(),
            },
        };
        assert!(filter.validate().is_err());
    }

    // ---- validate: Transfer with invalid original clauses ----

    #[test]
    fn validate_transfer_with_invalid_original_clauses_fails() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut eq = BTreeMap::new();
        eq.insert(
            "badField".to_string(),
            WhereClause {
                field: "badField".to_string(),
                operator: WhereOperator::Equal,
                value: Value::I64(1),
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
        assert!(filter.validate().is_err());
    }

    // ---- validate: UpdatePrice with invalid original clauses ----

    #[test]
    fn validate_update_price_with_invalid_original_clauses_fails() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut eq = BTreeMap::new();
        eq.insert(
            "badField".to_string(),
            WhereClause {
                field: "badField".to_string(),
                operator: WhereOperator::Equal,
                value: Value::I64(1),
            },
        );

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                price_clause: None,
            },
        };
        assert!(filter.validate().is_err());
    }

    // ---- validate: Purchase with invalid original clauses ----

    #[test]
    fn validate_purchase_with_invalid_original_clauses_fails() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let mut eq = BTreeMap::new();
        eq.insert(
            "badField".to_string(),
            WhereClause {
                field: "badField".to_string(),
                operator: WhereOperator::Equal,
                value: Value::I64(1),
            },
        );

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Purchase {
                original_document_clauses: InternalClauses {
                    equal_clauses: eq,
                    ..Default::default()
                },
                owner_clause: None,
            },
        };
        assert!(filter.validate().is_err());
    }

    // ---- evaluate_clauses: starts_with on field ----

    #[test]
    fn evaluate_clauses_starts_with_range_clause() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let internal_clauses = InternalClauses {
            range_clause: Some(WhereClause {
                field: "name".to_string(),
                operator: WhereOperator::StartsWith,
                value: Value::Text("Ali".to_string()),
            }),
            ..Default::default()
        };

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Create {
                new_document_clauses: internal_clauses.clone(),
            },
        };

        let id_value: Value = Identifier::from([1u8; 32]).into();

        let mut matching = BTreeMap::new();
        matching.insert("name".to_string(), Value::Text("Alice".to_string()));
        assert!(filter.evaluate_clauses(&internal_clauses, &id_value, &matching, None));

        let mut non_matching = BTreeMap::new();
        non_matching.insert("name".to_string(), Value::Text("Bob".to_string()));
        assert!(!filter.evaluate_clauses(&internal_clauses, &id_value, &non_matching, None));
    }

    // ---- validate: price clause between_exclude variants ----

    #[test]
    fn validate_price_clause_between_exclude_left_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::BetweenExcludeLeft,
                    value: Value::Array(vec![Value::U64(5), Value::U64(50)]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_between_exclude_right_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::BetweenExcludeRight,
                    value: Value::Array(vec![Value::U64(5), Value::U64(50)]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    #[test]
    fn validate_price_clause_between_exclude_bounds_valid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::UpdatePrice {
                original_document_clauses: InternalClauses::default(),
                price_clause: Some(ValueClause {
                    operator: WhereOperator::BetweenExcludeBounds,
                    value: Value::Array(vec![Value::U64(5), Value::U64(50)]),
                }),
            },
        };
        assert!(filter.validate().is_valid());
    }

    // ---- validate: transfer In with non-array ----

    #[test]
    fn validate_transfer_owner_clause_in_with_non_array_is_invalid() {
        let fixture = get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version);
        let contract = fixture.data_contract_owned();

        let filter = DriveDocumentQueryFilter {
            contract: &contract,
            document_type_name: "niceDocument".to_string(),
            action_clauses: DocumentActionMatchClauses::Transfer {
                original_document_clauses: InternalClauses::default(),
                owner_clause: Some(ValueClause {
                    operator: WhereOperator::In,
                    value: Value::Identifier([1u8; 32]),
                }),
            },
        };
        assert!(filter.validate().is_err());
    }

    mod canonical_values {
        use super::*;
        use dpp::platform_value::string_encoding::Encoding;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::DocumentCreateTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransition;
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::DocumentTransferTransitionV0;
        use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransition;

        fn contract() -> DataContract {
            get_data_contract_fixture(None, 0, LATEST_PLATFORM_VERSION.protocol_version)
                .data_contract_owned()
        }

        fn create(
            contract: &DataContract,
            document_type_name: &str,
            data: BTreeMap<String, Value>,
        ) -> DocumentTransition {
            DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                    id: Identifier::from([3u8; 32]),
                    document_type_name: document_type_name.to_string(),
                    data_contract_id: contract.id(),
                    identity_contract_nonce: 0,
                    token_payment_info: None,
                }),
                entropy: [0u8; 32],
                data,
                prefunded_voting_balance: None,
            }))
        }

        fn equal(field: &str, value: Value) -> InternalClauses {
            InternalClauses {
                equal_clauses: BTreeMap::from([(
                    field.to_string(),
                    WhereClause {
                        field: field.to_string(),
                        operator: WhereOperator::Equal,
                        value,
                    },
                )]),
                ..Default::default()
            }
        }

        #[test]
        fn should_match_an_identifier_encoded_differently_in_clause_and_transition() {
            let contract = contract();
            let platform_version = PlatformVersion::latest();
            let identifier = Identifier::from([7u8; 32]);
            // The clause names the identifier in base58, the transition carries raw bytes.
            let mut filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "withByteArrays".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: equal(
                        "identifierField",
                        Value::Text(identifier.to_string(Encoding::Base58)),
                    ),
                },
            };
            let transition = create(
                &contract,
                "withByteArrays",
                BTreeMap::from([
                    ("byteArrayField".to_string(), Value::Bytes(vec![1u8; 10])),
                    (
                        "identifierField".to_string(),
                        Value::Bytes(identifier.to_vec()),
                    ),
                ]),
            );

            filter
                .canonicalize_clause_values(platform_version)
                .expect("expected the clause value to canonicalize");
            assert!(filter.validate().is_valid());
            assert_eq!(
                filter.matches_document_transition(&transition, None, platform_version),
                TransitionCheckResult::Pass
            );

            let other = create(
                &contract,
                "withByteArrays",
                BTreeMap::from([("identifierField".to_string(), Value::Bytes(vec![8u8; 32]))]),
            );
            assert_eq!(
                filter.matches_document_transition(&other, None, platform_version),
                TransitionCheckResult::Fail
            );
        }

        #[test]
        fn should_match_a_value_longer_than_an_index_key() {
            let contract = contract();
            let platform_version = PlatformVersion::latest();
            let long_name = "x".repeat(300);
            let mut filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "niceDocument".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: equal("name", Value::Text(long_name.clone())),
                },
            };
            filter
                .canonicalize_clause_values(platform_version)
                .expect("expected the clause value to canonicalize");

            let transition = create(
                &contract,
                "niceDocument",
                BTreeMap::from([("name".to_string(), Value::Text(long_name))]),
            );
            assert_eq!(
                filter.matches_document_transition(&transition, None, platform_version),
                TransitionCheckResult::Pass
            );
        }

        #[test]
        fn should_reject_a_value_that_does_not_fit_its_field() {
            let contract = contract();
            let mut filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "withByteArrays".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: equal("identifierField", Value::Bytes(vec![1u8; 5])),
                },
            };
            assert!(matches!(
                filter.canonicalize_clause_values(PlatformVersion::latest()),
                Err(QuerySyntaxError::InvalidFormatWhereClause(_))
            ));
        }

        #[test]
        fn should_reject_clauses_on_system_fields_other_than_id() {
            let contract = contract();
            let owner: Value = Identifier::from([1u8; 32]).into();
            for field in ["$ownerId", "$createdAt"] {
                let filter = DriveDocumentQueryFilter {
                    contract: &contract,
                    document_type_name: "niceDocument".to_string(),
                    action_clauses: DocumentActionMatchClauses::Create {
                        new_document_clauses: equal(field, owner.clone()),
                    },
                };
                assert!(
                    !filter.validate().is_valid(),
                    "a clause on {field} can never match a transition"
                );
            }
        }

        #[test]
        fn should_match_a_transfer_recipient_given_as_bytes() {
            let contract = contract();
            let platform_version = PlatformVersion::latest();
            let recipient = Identifier::from([9u8; 32]);
            let mut filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "niceDocument".to_string(),
                action_clauses: DocumentActionMatchClauses::Transfer {
                    original_document_clauses: InternalClauses::default(),
                    owner_clause: Some(ValueClause {
                        operator: WhereOperator::In,
                        value: Value::Array(vec![Value::Bytes(recipient.to_vec())]),
                    }),
                },
            };
            filter
                .canonicalize_clause_values(platform_version)
                .expect("expected the recipient to canonicalize");
            assert!(filter.validate().is_valid());

            let transfer = DocumentTransition::Transfer(DocumentTransferTransition::V0(
                DocumentTransferTransitionV0 {
                    base: DocumentBaseTransition::V1(DocumentBaseTransitionV1 {
                        id: Identifier::from([3u8; 32]),
                        document_type_name: "niceDocument".to_string(),
                        data_contract_id: contract.id(),
                        identity_contract_nonce: 0,
                        token_payment_info: None,
                    }),
                    revision: 2,
                    recipient_owner_id: recipient,
                },
            ));
            assert_eq!(
                filter.matches_document_transition(&transfer, None, platform_version),
                TransitionCheckResult::Pass
            );
        }

        #[test]
        fn should_refuse_identifier_text_longer_than_any_identifier() {
            let contract = contract();
            for field in ["identifierField", "$id"] {
                let clauses = if field == "$id" {
                    InternalClauses {
                        primary_key_equal_clause: Some(WhereClause {
                            field: field.to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::Text("z".repeat(120_000)),
                        }),
                        ..Default::default()
                    }
                } else {
                    equal(field, Value::Text("z".repeat(120_000)))
                };
                let mut filter = DriveDocumentQueryFilter {
                    contract: &contract,
                    document_type_name: "withByteArrays".to_string(),
                    action_clauses: DocumentActionMatchClauses::Delete {
                        original_document_clauses: clauses,
                    },
                };
                assert!(
                    filter
                        .canonicalize_clause_values(PlatformVersion::latest())
                        .is_err(),
                    "{field}"
                );
            }
        }

        #[test]
        fn should_keep_empty_byte_array_operands() {
            let contract = contract();
            let platform_version = PlatformVersion::latest();
            for (operator, value) in [
                (WhereOperator::Equal, Value::Bytes(vec![])),
                (
                    WhereOperator::In,
                    Value::Array(vec![Value::Bytes(vec![]), Value::Bytes(vec![1])]),
                ),
            ] {
                let mut filter = DriveDocumentQueryFilter {
                    contract: &contract,
                    document_type_name: "withByteArrays".to_string(),
                    action_clauses: DocumentActionMatchClauses::Create {
                        new_document_clauses: InternalClauses {
                            equal_clauses: if operator == WhereOperator::Equal {
                                BTreeMap::from([(
                                    "byteArrayField".to_string(),
                                    WhereClause {
                                        field: "byteArrayField".to_string(),
                                        operator,
                                        value: value.clone(),
                                    },
                                )])
                            } else {
                                BTreeMap::new()
                            },
                            in_clauses: if operator == WhereOperator::In {
                                vec![WhereClause {
                                    field: "byteArrayField".to_string(),
                                    operator,
                                    value: value.clone(),
                                }]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        },
                    },
                };
                filter
                    .canonicalize_clause_values(platform_version)
                    .expect("an empty byte array is a byte array");
                assert!(filter.validate().is_valid(), "{operator:?}");
                let transition = create(
                    &contract,
                    "withByteArrays",
                    BTreeMap::from([("byteArrayField".to_string(), Value::Bytes(vec![]))]),
                );
                assert_eq!(
                    filter.matches_document_transition(&transition, None, platform_version),
                    TransitionCheckResult::Pass,
                    "{operator:?}"
                );
            }
        }

        /// A derived index property such as `postId.$ownerId` resolves to an identifier but
        /// is not a schema property; like any field outside the schema, it is refused before
        /// a codec could decode a hostile operand.
        #[test]
        fn should_refuse_fields_outside_the_schema_before_decoding_them() {
            let contract = contract();
            for operator in [WhereOperator::Equal, WhereOperator::In] {
                let long = Value::Text("z".repeat(120_000));
                let clause = WhereClause {
                    field: "postId.$ownerId".to_string(),
                    operator,
                    value: if operator == WhereOperator::In {
                        Value::Array(vec![long])
                    } else {
                        long
                    },
                };
                let mut filter = DriveDocumentQueryFilter {
                    contract: &contract,
                    document_type_name: "niceDocument".to_string(),
                    action_clauses: DocumentActionMatchClauses::Create {
                        new_document_clauses: InternalClauses {
                            equal_clauses: if operator == WhereOperator::Equal {
                                BTreeMap::from([(clause.field.clone(), clause.clone())])
                            } else {
                                BTreeMap::new()
                            },
                            in_clauses: if operator == WhereOperator::In {
                                vec![clause]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        },
                    },
                };
                assert!(
                    filter
                        .canonicalize_clause_values(PlatformVersion::latest())
                        .is_err(),
                    "{operator:?}"
                );
            }
        }

        #[test]
        fn should_tell_empty_text_from_a_nul_character() {
            let contract = contract();
            let platform_version = PlatformVersion::latest();
            let filter_for = |clauses: InternalClauses| DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "niceDocument".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: clauses,
                },
            };
            let name = |text: &str| {
                create(
                    &contract,
                    "niceDocument",
                    BTreeMap::from([("name".to_string(), Value::Text(text.to_string()))]),
                )
            };

            let mut empty = filter_for(equal("name", Value::Text(String::new())));
            empty
                .canonicalize_clause_values(platform_version)
                .expect("canonical");
            assert_eq!(
                empty.matches_document_transition(&name(""), None, platform_version),
                TransitionCheckResult::Pass
            );
            assert_eq!(
                empty.matches_document_transition(&name("\0"), None, platform_version),
                TransitionCheckResult::Fail
            );

            let mut both = filter_for(InternalClauses {
                in_clauses: vec![WhereClause {
                    field: "name".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::Text(String::new()),
                        Value::Text("\0".to_string()),
                    ]),
                }],
                ..Default::default()
            });
            both.canonicalize_clause_values(platform_version)
                .expect("canonical");
            assert!(both.validate().is_valid(), "two distinct candidates");

            let mut prefix = filter_for(InternalClauses {
                range_clause: Some(WhereClause {
                    field: "name".to_string(),
                    operator: WhereOperator::StartsWith,
                    value: Value::Text("\0".to_string()),
                }),
                ..Default::default()
            });
            prefix
                .canonicalize_clause_values(platform_version)
                .expect("canonical");
            assert!(prefix.validate().is_valid(), "a non-empty prefix");
        }

        #[test]
        fn should_accept_packed_byte_candidates_for_a_u8_field() {
            use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
            use dpp::data_contract::DataContractFactory;
            use dpp::platform_value::platform_value;
            let documents = platform_value!({
                "rating": {
                    "type": "object",
                    "properties": {
                        "stars": { "type": "integer", "minimum": 0, "maximum": 255, "position": 0 }
                    },
                    "additionalProperties": false
                }
            });
            let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
                .expect("factory")
                .create_with_value_config(Identifier::from([1u8; 32]), 1, documents, None, None)
                .expect("the contract parses")
                .data_contract_owned();
            let stars_type = contract
                .document_type_for_name("rating")
                .expect("type")
                .flattened_properties()
                .get("stars")
                .expect("stars")
                .property_type
                .clone();
            assert_eq!(stars_type, DocumentPropertyType::U8, "fixture field is u8");

            let platform_version = PlatformVersion::latest();
            let mut filter = DriveDocumentQueryFilter {
                contract: &contract,
                document_type_name: "rating".to_string(),
                action_clauses: DocumentActionMatchClauses::Create {
                    new_document_clauses: InternalClauses {
                        in_clauses: vec![WhereClause {
                            field: "stars".to_string(),
                            operator: WhereOperator::In,
                            value: Value::Bytes(vec![4, 5]),
                        }],
                        ..Default::default()
                    },
                },
            };
            filter
                .canonicalize_clause_values(platform_version)
                .expect("packed u8 candidates");
            assert!(filter.validate().is_valid());
            for (stars, expected) in [
                (Value::U64(5), TransitionCheckResult::Pass),
                (Value::U64(3), TransitionCheckResult::Fail),
            ] {
                let transition = create(
                    &contract,
                    "rating",
                    BTreeMap::from([("stars".to_string(), stars)]),
                );
                assert_eq!(
                    filter.matches_document_transition(&transition, None, platform_version),
                    expected
                );
            }
        }
    }
}
