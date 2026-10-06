//! Protocol v14 generation of document type update validation.
//!
//! v0 validated index changes by comparing `IndexLevel` trees whose
//! `level_identifier`s are assigned by an incrementing counter while walking
//! `indices` — a BTreeMap keyed by index NAME. Adding an index whose name
//! sorted before an existing one renumbered every level, so the identifier
//! equality check rejected the update with an opaque "Invalid path", while
//! the semantically identical addition under a late-sorting name passed the
//! tree comparison (and then hard-errored in the JSON-schema compatibility
//! check, which has no rule for the `indices` keyword). Which consensus
//! outcome a contract owner got therefore depended purely on how the new
//! index's name sorted.
//!
//! v1 drops the tree comparison and compares the parsed index definitions
//! by name instead: any added, removed or modified index is rejected with a
//! deterministic `DataContractInvalidIndexDefinitionUpdateError` naming the
//! offending index, independent of name sort order. This does not change
//! which updates are ultimately acceptable — under v0 no index modification
//! could ever pass the full pipeline (whatever survived the tree comparison
//! was always rejected by the `indices` schema-compatibility hard error) —
//! it makes the rejection deterministic, clean, and correctly labeled.

use crate::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, DataContractInvalidRequiredFieldsUpdateError,
};
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::config::moderation::SettledDeletionRule;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV1Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeSet;

use super::common::UpdateValidationOptions;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Legacy keep-history types advertised deletes that Drive never allowed.
        // Permit only true -> false for that flag while keeping history enabled.
        // Every other config and schema check still runs, and v0 stays immutable.
        let options = UpdateValidationOptions {
            allow_history_delete_repair: self.documents_keep_history()
                && new_document_type.documents_keep_history()
                && self.documents_can_be_deleted()
                && !new_document_type.documents_can_be_deleted(),
        };
        let result = self.validate_config_with_options(new_document_type, &options);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the type keeps what its moderators may do to its
        // documents (a generation 1 rule: the keyword arrives with protocol
        // version 14, and the shared config checks above also serve v0)
        let result = self.validate_moderator_abilities_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the type keeps its time to live (the keyword arrives with
        // protocol version 14, the only version selecting this generation)
        let result = self.validate_documents_ttl_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the type keeps the replace its barred owners may still make (the
        // keyword arrives with protocol version 14, the only version selecting this generation)
        let result = self.validate_retracted_when_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the type keeps whether only a consume deletes its documents (the
        // value arrives with protocol version 14, the only version selecting this generation)
        let result = self.validate_deleted_only_when_consumed_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that a property the update adds is generated only when one of its
        // params is new too (the keyword arrives with protocol version 14, the only
        // version selecting this generation)
        let result = self.validate_generated_from_additions(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that index definitions are unchanged
        let result = self.validate_index_definitions_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no byte array property changes its on-disk encoding
        let result = self.validate_byte_array_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no integer property changes its width or signedness
        let result = self.validate_integer_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no typed array changes how its elements are encoded
        let result = self.validate_typed_array_element_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate required-field changes (the schema compatibility differ
        // has the top-level `required` key stripped, so this is the only
        // place top-level requiredness changes are judged)
        let result = self.validate_required_fields_update(new_document_type, new_contract_version);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate immutable-property changes (the schema compatibility
        // differ has the top-level `immutable` key stripped, so this is the
        // only place those changes are judged)
        let result = self.validate_immutable_fields_update(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the action fees are unchanged
        let result = self.validate_action_fees_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that the token costs are unchanged
        let result = self.validate_token_costs_unchanged(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema_with_options(new_document_type, platform_version, &options)
    }

    /// An integer property is stored at the width and signedness of its type,
    /// in the document and in every index key on it, and the type comes from
    /// the property's `minimum` and `maximum`, or from its `enum` values when
    /// it has no bounds (with `sizedIntegerTypes` on; off, every integer is an
    /// i64). The schema compatibility rules allow each change that moves it:
    /// raising `maximum`, lowering `minimum`, removing either, adding `enum`
    /// values, and so does turning `sizedIntegerTypes` on. Documents already
    /// stored then no longer decode, or decode to other values, and their
    /// index entries sit under keys of the old width. So the type is held
    /// here, as `validate_byte_array_encoding_stability` holds a byte array
    /// property's; a bound change that keeps the type is still allowed. The
    /// signedness is held with the width, even where the old bounds keep every
    /// stored value readable both ways (a u8 capped at 100 read as an i8):
    /// nothing bounds a u64 that becomes an i64, and one comparison is the
    /// whole rule. A change to a non-integer type is the schema compatibility
    /// check's to refuse.
    fn validate_integer_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            if !old_property.property_type.is_integer() {
                continue;
            }
            let Some(new_property) = new_properties.get(path) else {
                continue;
            };
            if !new_property.property_type.is_integer() {
                continue;
            }

            let old_encoding = old_property.property_type.stored_encoding();
            let new_encoding = new_property.property_type.stored_encoding();
            if old_encoding != new_encoding {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the integer encoding of property \
                             '{}': its values are stored as {} and would be read as {}",
                            path, old_encoding, new_encoding,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// A typed array stores each element exactly as a required scalar property
    /// of its element type is stored, so an update that changes how an
    /// element encodes would misread every element already stored: an
    /// integer element whose width changes (its bounds or `enum` choose it),
    /// or a byte array element that turns from fixed-size (raw) to variable
    /// (length-prefixed) or to another fixed size. The schema compatibility
    /// rules allow the changes that do this (raising `maximum`, widening
    /// `maxItems`), so the element encoding is held here, as
    /// `validate_byte_array_encoding_stability` holds a byte array
    /// property's. Every other element change, a longer `maxLength` included,
    /// leaves the encoding as it is.
    fn validate_typed_array_element_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            let DocumentPropertyType::TypedArray(old_array) = &old_property.property_type else {
                continue;
            };
            let Some(new_property) = new_properties.get(path) else {
                continue;
            };
            let DocumentPropertyType::TypedArray(new_array) = &new_property.property_type else {
                continue;
            };

            let old_encoding = old_array.item_type.stored_encoding();
            let new_encoding = new_array.item_type.stored_encoding();
            if old_encoding != new_encoding {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the element encoding of typed array \
                             property '{}': its elements are stored as {} and would be read as \
                             {}",
                            path, old_encoding, new_encoding,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// The action fees of a document type are fixed when it is published: an
    /// update may not add, change or remove them, nor switch their pricing.
    /// A document type added by the update is not judged here and may declare
    /// its own. A transition names the fee it agrees to pay (its action fee
    /// agreement), so lifting this later cannot make a signed transition pay
    /// a fee its signer never saw.
    fn validate_action_fees_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        if self.action_fees() == new_document_type.action_fees() {
            return SimpleConsensusValidationResult::new();
        }
        let change = match (self.action_fees(), new_document_type.action_fees()) {
            (None, Some(_)) => "add",
            (Some(_), None) => "remove",
            _ => "change",
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not {change} its action fees: they are fixed when the \
                     document type is published"
                ),
            )
            .into(),
        )
    }

    /// The token costs of a document type are fixed when it is published, as
    /// its action fees are: whoever holds a document of the type got it
    /// knowing what replacing, deleting, transferring or selling it costs in
    /// tokens, which token, and who pays the gas. An update may not add,
    /// change or remove the cost of any action. A document type added by the
    /// update is not judged here and may declare its own. No earlier protocol
    /// version let an update change them either: the schema compatibility
    /// differ failed on any `tokenCost` diff.
    fn validate_token_costs_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let costs = [
            (
                "create",
                self.document_creation_token_cost(),
                new_document_type.document_creation_token_cost(),
            ),
            (
                "replace",
                self.document_replacement_token_cost(),
                new_document_type.document_replacement_token_cost(),
            ),
            (
                "delete",
                self.document_deletion_token_cost(),
                new_document_type.document_deletion_token_cost(),
            ),
            (
                "transfer",
                self.document_transfer_token_cost(),
                new_document_type.document_transfer_token_cost(),
            ),
            (
                "update_price",
                self.document_update_price_token_cost(),
                new_document_type.document_update_price_token_cost(),
            ),
            (
                "purchase",
                self.document_purchase_token_cost(),
                new_document_type.document_purchase_token_cost(),
            ),
        ];
        for (action, old_cost, new_cost) in costs {
            if old_cost == new_cost {
                continue;
            }
            let change = match (old_cost, new_cost) {
                (None, Some(_)) => "add",
                (Some(_), None) => "remove",
                _ => "change",
            };
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not {change} the token cost of its {action} action: \
                         token costs are fixed when the document type is published"
                    ),
                )
                .into(),
            );
        }
        SimpleConsensusValidationResult::new()
    }

    /// What moderators may do to documents of a type (`moderatorAbilities`) is
    /// fixed when the type is created: whoever wrote a document knows from the
    /// type's first version who may take it down and which of its fields
    /// others write, a type that is the target of a permanentDocument
    /// reference was admitted as one nobody can delete, and a field only
    /// moderators write starts absent on every document, so no stored one could
    /// hold a value its owner set, and which fields stay public in the record of
    /// a moderator's deletion is what an author was told when writing. A
    /// document type added by an update declares the abilities freely.
    fn validate_moderator_abilities_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let (old_fields, new_fields) = (
            self.moderator_changeable_fields(),
            new_document_type.moderator_changeable_fields(),
        );
        if old_fields != new_fields {
            let list = |fields: &BTreeSet<String>| {
                if fields.is_empty() {
                    "none".to_string()
                } else {
                    fields.iter().cloned().collect::<Vec<_>>().join(", ")
                }
            };
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change which fields only its moderators write: changing from {} to {}",
                        list(old_fields),
                        list(new_fields)
                    ),
                )
                .into(),
            );
        }
        let (old_kept, new_kept) = (
            self.moderator_deletion_kept_fields(),
            new_document_type.moderator_deletion_kept_fields(),
        );
        if old_kept != new_kept {
            let list = |fields: &BTreeSet<String>| {
                if fields.is_empty() {
                    "none".to_string()
                } else {
                    fields.iter().cloned().collect::<Vec<_>>().join(", ")
                }
            };
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change which fields a moderator's removal record keeps: changing from {} to {}",
                        list(old_kept),
                        list(new_kept)
                    ),
                )
                .into(),
            );
        }
        if new_document_type.documents_can_be_deleted_by_moderators()
            == self.documents_can_be_deleted_by_moderators()
        {
            // The window the moderators have is fixed with the flag: a longer one would
            // reopen documents that had settled, and one rule for both directions keeps
            // what an author was told when they wrote.
            let (old_window, new_window) = (
                self.documents_can_be_deleted_by_moderators_for(),
                new_document_type.documents_can_be_deleted_by_moderators_for(),
            );
            if old_window == new_window {
                // What a deletion leaves is fixed with it: the removal records tree exists
                // exactly for a type that keeps them, and an owner who wrote under a refund
                // keeps it.
                for (what, old, new) in [
                    (
                        "whether a moderator's deletion leaves a removal record",
                        self.moderator_deletions_keep_records(),
                        new_document_type.moderator_deletions_keep_records(),
                    ),
                    (
                        "whether a moderator's deletion refunds the document's owner",
                        self.moderator_deletions_refund_owner(),
                        new_document_type.moderator_deletions_refund_owner(),
                    ),
                ] {
                    if old != new {
                        return SimpleConsensusValidationResult::new_with_error(
                            DocumentTypeUpdateError::new(
                                self.data_contract_id(),
                                self.name(),
                                format!(
                                "document type can not change {what}: changing from {old} to {new}"
                            ),
                            )
                            .into(),
                        );
                    }
                }
                // Who must approve the deletion of a settled document is fixed with the
                // window: fewer approvals would reach content its author wrote under more.
                let (old_rule, new_rule) = (
                    self.moderator_settled_deletion(),
                    new_document_type.moderator_settled_deletion(),
                );
                if old_rule != new_rule {
                    let rule = |rule: Option<SettledDeletionRule>| match rule {
                        None => "no deletion once settled".to_string(),
                        Some(SettledDeletionRule {
                            leader,
                            approvals,
                            approvers_predate_document,
                        }) => format!(
                            "{approvals} approvals{}{}",
                            if leader {
                                ", the team's leader among them"
                            } else {
                                ""
                            },
                            if approvers_predate_document {
                                ", added members only from before the document"
                            } else {
                                ", added members whenever added"
                            }
                        ),
                    };
                    return SimpleConsensusValidationResult::new_with_error(
                        DocumentTypeUpdateError::new(
                            self.data_contract_id(),
                            self.name(),
                            format!(
                                "document type can not change who must approve a moderator's deletion of a settled document: changing from {} to {}",
                                rule(old_rule),
                                rule(new_rule)
                            ),
                        )
                        .into(),
                    );
                }
                return SimpleConsensusValidationResult::new();
            }
            let seconds = |window: Option<u32>| {
                window.map_or("no limit".to_string(), |seconds| {
                    format!("{seconds} seconds")
                })
            };
            return SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(
                    self.data_contract_id(),
                    self.name(),
                    format!(
                        "document type can not change for how long after a document's last modification moderators can delete it: changing from {} to {}",
                        seconds(old_window),
                        seconds(new_window)
                    ),
                )
                .into(),
            );
        }
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not change whether its documents can be deleted by moderators: changing from {} to {}",
                    self.documents_can_be_deleted_by_moderators(),
                    new_document_type.documents_can_be_deleted_by_moderators()
                ),
            )
            .into(),
        )
    }

    /// A property this update adds may declare `generatedFrom` only when a param is new
    /// too. Documents stored before the update were never generated, so a new generated
    /// property whose params all existed would be missing from every stored document
    /// holding them, for good on a type whose documents are never replaced; with a new
    /// param, those documents lack it, and the generated property is rightly absent. A
    /// property that already existed keeps its declaration unchanged: the schema
    /// compatibility differ freezes the keyword.
    fn validate_generated_from_additions(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let old_properties = self.flattened_properties();
        for (path, generated_from) in new_document_type.generated_from_fields() {
            if old_properties.contains_key(path) {
                continue;
            }
            if generated_from
                .property_params()
                .all(|param| old_properties.contains_key(param))
            {
                let params = generated_from.params_description();
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not add property \"{path}\" generated from existing properties ({params}): documents stored before the update hold them without it"
                        ),
                    )
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }

    /// Whether only a consume deletes the documents of a type (`canBeDeleted:
    /// "onlyWhenConsumed"`) is fixed when the type is created. Turning it on would let
    /// documents of a type that promised never to lose them leave state, under the
    /// `permanentDocument` references made to it; turning it off would strand the
    /// references that consume them. A change from `true` is already refused as a change of
    /// whether the owner may delete; this catches the change from `false`, which the owner's
    /// flag does not see. It runs before the schema compatibility differ, which only freezes
    /// the key's text, so a real change gets this error.
    fn validate_deleted_only_when_consumed_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let (old, new) = (
            self.documents_deleted_only_when_consumed(),
            new_document_type.documents_deleted_only_when_consumed(),
        );
        if old == new {
            return SimpleConsensusValidationResult::new();
        }
        let describe = |only_when_consumed: bool, can_be_deleted: bool| {
            if only_when_consumed {
                "\"onlyWhenConsumed\"".to_string()
            } else {
                can_be_deleted.to_string()
            }
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not change whether only a consume deletes its documents: changing canBeDeleted from {} to {}",
                    describe(old, self.documents_can_be_deleted()),
                    describe(new, new_document_type.documents_can_be_deleted())
                ),
            )
            .into(),
        )
    }

    /// A document type's time to live is fixed when the type is created. Every document
    /// already stored has its expiry indexed from the time to live it was written with, and
    /// paid for that lifetime: adding a `ttl` would leave the stored documents without an
    /// entry the cleanup could find, removing it would leave entries deleting documents
    /// the type says live forever, and changing it would move expiries nobody paid for.
    /// It runs before the schema compatibility differ, which only freezes the key's text,
    /// so a real change gets this error. A document type added by an update declares `ttl`
    /// freely.
    fn validate_documents_ttl_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let (old_ttl, new_ttl) = (
            self.documents_ttl_seconds(),
            new_document_type.documents_ttl_seconds(),
        );
        if old_ttl == new_ttl {
            return SimpleConsensusValidationResult::new();
        }
        let describe = |ttl: Option<u32>| {
            ttl.map_or("no time to live".to_string(), |seconds| {
                format!("{seconds} seconds")
            })
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not change the time to live of its documents: changing from {} to {}",
                    describe(old_ttl),
                    describe(new_ttl)
                ),
            )
            .into(),
        )
    }

    /// What a document type's `retractedWhen` lets a banned or suspended owner write is
    /// fixed when the type is created. Loosening it would let barred owners write what
    /// their bar was imposed under the promise of refusing, and tightening or removing it
    /// would take back the one exit an author whose documents can not be deleted was given
    /// when it wrote them. Whether one condition holds wherever another does can not be
    /// told in general, so any change is refused. It runs before the schema compatibility
    /// differ, which only freezes the key's text, so a real change gets this error. A
    /// document type added by an update declares `retractedWhen` freely.
    fn validate_retracted_when_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        if self.retracted_when() == new_document_type.retracted_when() {
            return SimpleConsensusValidationResult::new();
        }
        let change = match (self.retracted_when(), new_document_type.retracted_when()) {
            (None, _) => "add",
            (_, None) => "remove",
            _ => "change",
        };
        SimpleConsensusValidationResult::new_with_error(
            DocumentTypeUpdateError::new(
                self.data_contract_id(),
                self.name(),
                format!(
                    "document type can not {change} its `retractedWhen` condition: what a banned \
                     or suspended owner may still write is fixed when the type is created"
                ),
            )
            .into(),
        )
    }

    /// Top-level requiredness may only change in one way: a brand-new
    /// property may be added as required when it is annotated with
    /// `requiredSince` equal to the contract version this update creates.
    /// Everything else is frozen: requiredness is baked into the document
    /// wire format (required properties serialize without a presence flag),
    /// and the per-document contract-version stamp resolves layouts from the
    /// latest schema alone only if annotations never change retroactively.
    ///
    /// Nested (dotted) required paths and the `requiredSince` keyword on
    /// existing properties stay frozen by the schema compatibility differ;
    /// this check judges the top-level `required` key, which is stripped
    /// from the diff exactly like `indices`.
    fn validate_required_fields_update(
        &self,
        new_document_type: DocumentTypeRef,
        new_contract_version: u32,
    ) -> SimpleConsensusValidationResult {
        let old_required = self.required_fields();
        let new_required = new_document_type.required_fields();

        for name in old_required {
            // Nested paths are governed by the schema compatibility differ
            if name.contains('.') {
                continue;
            }
            if !new_required.contains(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("removed required field '{name}'"),
                    )
                    .into(),
                );
            }
        }

        for name in new_required {
            if name.contains('.') || old_required.contains(name) {
                continue;
            }
            if name.starts_with('$') {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("system field '{name}' cannot become required"),
                    )
                    .into(),
                );
            }
            if self.properties().contains_key(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("existing property '{name}' cannot become required"),
                    )
                    .into(),
                );
            }
            let Some(new_property) = new_document_type.properties().get(name) else {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!("added required field '{name}' references an unknown property"),
                    )
                    .into(),
                );
            };
            if new_property.required_since != Some(new_contract_version) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        self.name().to_string(),
                        format!(
                            "new required field '{name}' must carry requiredSince {new_contract_version}, the contract version this update creates"
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// What `immutable` freezes may only tighten. The properties it lists
    /// without a condition may only grow: removing one would let a later
    /// replace change a property documents were created under the promise of
    /// never changing, while adding one only narrows what future replaces may
    /// touch and invalidates no stored document (the parser has already
    /// checked that every new entry names a top-level property of the new
    /// type). A property listed with a condition keeps it as it is, or loses
    /// it to be listed without one, which freezes it whatever the condition
    /// says. Whether one condition holds wherever another does can not be
    /// told in general, so a changed condition is refused, as is a property
    /// dropped from the list. A property the list did not hold may be added
    /// with a condition, which only narrows what replaces may touch.
    ///
    /// Judged here because the top-level key is stripped from the schema
    /// compatibility diff, exactly like `indices` and `required`.
    fn validate_immutable_fields_update(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let new_immutable = new_document_type.immutable_fields();
        let new_conditions = new_document_type.immutable_field_conditions();
        let refused = |message: String| {
            SimpleConsensusValidationResult::new_with_error(
                DocumentTypeUpdateError::new(self.data_contract_id(), self.name(), message).into(),
            )
        };

        for property in self.immutable_fields() {
            if !new_immutable.contains(property) {
                return refused(format!(
                    "document type can not remove immutable property '{property}': the \
                     properties immutable lists without a condition may only grow"
                ));
            }
        }

        for (property, condition) in self.immutable_field_conditions() {
            if new_immutable.contains(property) {
                continue;
            }
            match new_conditions.get(property) {
                Some(new_condition) if new_condition == condition => {}
                Some(_) => {
                    return refused(format!(
                        "document type can not change the condition under which immutable \
                         property '{property}' is frozen: it may only be listed without one"
                    ))
                }
                None => {
                    return refused(format!(
                        "document type can not remove immutable property '{property}': a \
                         property listed with a condition may only be listed without one"
                    ))
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// Index definitions are immutable once a document type is registered:
    /// Drive lays out the index trees at contract creation and never
    /// backfills them, so an added index would silently miss every
    /// pre-update document and a removed or modified one would orphan
    /// on-disk subtrees. Compare the definitions by index name — the
    /// comparison must not depend on where a changed index's name sorts
    /// relative to the document type's other indexes.
    fn validate_index_definitions_unchanged(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        let old_indexes = self.indexes();
        let new_indexes = new_document_type.indexes();

        for (name, old_index) in old_indexes {
            match new_indexes.get(name) {
                None => {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidIndexDefinitionUpdateError::new(
                            self.name().to_string(),
                            format!("removed index '{name}'"),
                        )
                        .into(),
                    );
                }
                Some(new_index) if new_index != old_index => {
                    return SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidIndexDefinitionUpdateError::new(
                            self.name().to_string(),
                            format!("changed index '{name}'"),
                        )
                        .into(),
                    );
                }
                _ => {}
            }
        }

        for name in new_indexes.keys() {
            if !old_indexes.contains_key(name) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidIndexDefinitionUpdateError::new(
                        self.name().to_string(),
                        format!("added index '{name}'"),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
    use crate::data_contract::config::moderation::{
        ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
        ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
    };
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use crate::validation::SimpleConsensusValidationResult;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::{BTreeMap, BTreeSet};

    fn doc_type_with_indices(indices: Value, platform_version: &PlatformVersion) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "indices": indices,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    fn old_doc_type(platform_version: &PlatformVersion) -> DocumentType {
        doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        )
    }

    /// A mutable document type with three string properties and the given
    /// `immutable` list.
    fn doc_type_with_immutable(
        immutable: Value,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "immutable": immutable,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // The `immutable` list may only grow: removing an entry would let a
    // later replace change a property documents were created under the
    // promise of never changing.
    #[test]
    fn should_return_invalid_result_when_can_be_deleted_by_moderators_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();

        let schema = |deletable_by_moderators: bool| {
            platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
                "moderatorAbilities": { "delete": deletable_by_moderators },
            })
        };

        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));

        let make_document_type = |schema: platform_value::Value| {
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Neither direction is allowed: authors keep the rules they wrote under, and a
        // permanentDocument reference was admitted against a type nobody can delete.
        for (old_flag, new_flag) in [(false, true), (true, false)] {
            let old_document_type = make_document_type(schema(old_flag));
            let new_document_type = make_document_type(schema(new_flag));

            // The whole generation 1 pipeline: the rule answers before the schema
            // compatibility differ, which only freezes the keyword's text.
            let result = old_document_type
                .as_ref()
                .validate_update(new_document_type.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            let expected = format!(
                "document type can not change whether its documents can be deleted by moderators: changing from {} to {}",
                old_flag, new_flag
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == expected
            );
        }
    }

    #[test]
    fn should_return_invalid_result_when_the_moderators_window_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let make_document_type = |window: Option<u32>| {
            let mut schema = platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "required": ["$updatedAt"],
                "additionalProperties": false,
                "moderatorAbilities": { "delete": true },
            });
            if let Some(seconds) = window {
                schema
                    .set_value_at_path(
                        "moderatorAbilities",
                        "deleteWithin",
                        platform_value::Value::U32(seconds),
                    )
                    .expect("expected to set the window");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Longer would reopen documents that had settled; shorter, given or taken away, would
        // still change what an author was told. One rule for every direction.
        for (old_window, new_window, from, to) in [
            (Some(86400), Some(172800), "86400 seconds", "172800 seconds"),
            (Some(86400), Some(3600), "86400 seconds", "3600 seconds"),
            (Some(86400), None, "86400 seconds", "no limit"),
            (None, Some(86400), "no limit", "86400 seconds"),
        ] {
            let result = make_document_type(old_window)
                .as_ref()
                .validate_update(make_document_type(new_window).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            let expected = format!(
                "document type can not change for how long after a document's last modification moderators can delete it: changing from {from} to {to}"
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(Some(86400))
            .as_ref()
            .validate_update(
                make_document_type(Some(86400)).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_what_a_moderators_deletion_leaves_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let make_document_type = |keeps_record: bool, refunds_owner: bool| {
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                    },
                    "additionalProperties": false,
                    "moderatorAbilities": {
                        "delete": true,
                        "deleteKeepsRecord": keeps_record,
                        "deleteRefundsOwner": refunds_owner,
                    },
                }),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // The records tree exists exactly for a type that keeps records, and an owner who wrote
        // under a refund keeps it: neither changes, in either direction.
        for ((old_record, old_refund), (new_record, new_refund), expected) in [
            (
                (true, false),
                (false, false),
                "document type can not change whether a moderator's deletion leaves a removal record: changing from true to false",
            ),
            (
                (false, false),
                (true, false),
                "document type can not change whether a moderator's deletion leaves a removal record: changing from false to true",
            ),
            (
                (true, false),
                (true, true),
                "document type can not change whether a moderator's deletion refunds the document's owner: changing from false to true",
            ),
            (
                (true, true),
                (true, false),
                "document type can not change whether a moderator's deletion refunds the document's owner: changing from true to false",
            ),
        ] {
            let result = make_document_type(old_record, old_refund)
                .as_ref()
                .validate_update(
                    make_document_type(new_record, new_refund).as_ref(),
                    2,
                    platform_version,
                )
                .expect("validate_update should not error");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(false, true)
            .as_ref()
            .validate_update(
                make_document_type(false, true).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_who_approves_a_settled_deletion_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::Elected(Box::new(ElectedModerators {
                    join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                    vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
                    challenge_cool_down: None,
                    election_delay: None,
                    max_added_moderators: 0,
                    moderated_document_types: BTreeMap::from([(
                        "post".to_string(),
                        BTreeSet::from([ModerationAbility::DeleteDocuments]),
                    )]),
                    interim: InterimModerators::ContractOwner,
                    owner_protected: false,
                })),
                warnings: false,
            }));
        let make_document_type = |rule: Option<Value>| {
            let mut abilities = platform_value!({ "delete": true, "deleteWithin": 86400 });
            if let Some(rule) = rule {
                abilities
                    .insert("deleteSettled".to_string(), rule)
                    .expect("expected to set the rule");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                    },
                    "required": ["$createdAt", "$updatedAt"],
                    "additionalProperties": false,
                    "moderatorAbilities": abilities,
                }),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Fewer approvals would reach content written under more, and more would take back
        // what the contract promised its moderators: the rule changes in no direction. Letting
        // members added after a document approve its deletion would let the leader add them.
        for (old, new, expected) in [
            (
                None,
                Some(platform_value!({ "leader": true })),
                "document type can not change who must approve a moderator's deletion of a settled document: changing from no deletion once settled to 1 approvals, the team's leader among them, added members whenever added",
            ),
            (
                Some(platform_value!({ "leader": true, "approvals": 3 })),
                Some(platform_value!({ "approvals": 3 })),
                "document type can not change who must approve a moderator's deletion of a settled document: changing from 3 approvals, the team's leader among them, added members only from before the document to 3 approvals, added members only from before the document",
            ),
            (
                Some(platform_value!({ "approvals": 2 })),
                None,
                "document type can not change who must approve a moderator's deletion of a settled document: changing from 2 approvals, added members only from before the document to no deletion once settled",
            ),
            (
                Some(platform_value!({ "approvals": 2 })),
                Some(platform_value!({ "approvals": 2, "approversPredateDocument": false })),
                "document type can not change who must approve a moderator's deletion of a settled document: changing from 2 approvals, added members only from before the document to 2 approvals, added members whenever added",
            ),
        ] {
            let result = make_document_type(old)
                .as_ref()
                .validate_update(make_document_type(new).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let rule = || Some(platform_value!({ "leader": true, "approvals": 3 }));
        let result = make_document_type(rule())
            .as_ref()
            .validate_update(make_document_type(rule()).as_ref(), 2, platform_version)
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_the_fields_a_removal_record_keeps_are_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let make_document_type = |kept: &[&str]| {
            let mut abilities = platform_value!({ "delete": true });
            if !kept.is_empty() {
                abilities
                    .insert("deleteKeepsFields".to_string(), platform_value!(kept))
                    .expect("expected to set the key");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "post",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                        "hashtag": { "type": "string", "maxLength": 61, "position": 1 },
                    },
                    "additionalProperties": false,
                    "moderatorAbilities": abilities,
                }),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Which fields stay public once a moderator removes a document is what its author was
        // told: neither more nor fewer, in either direction.
        for (old, new, expected) in [
            (
                &[][..],
                &["hashtag"][..],
                "document type can not change which fields a moderator's removal record keeps: changing from none to hashtag",
            ),
            (
                &["hashtag"][..],
                &[][..],
                "document type can not change which fields a moderator's removal record keeps: changing from hashtag to none",
            ),
            (
                &["hashtag"][..],
                &["hashtag", "text"][..],
                "document type can not change which fields a moderator's removal record keeps: changing from hashtag to hashtag, text",
            ),
        ] {
            let result = make_document_type(old)
                .as_ref()
                .validate_update(make_document_type(new).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(&["hashtag"])
            .as_ref()
            .validate_update(
                make_document_type(&["hashtag"]).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_the_fields_only_moderators_write_are_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let make_document_type = |fields: &[&str]| {
            let mut schema = platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "status": { "type": "integer", "minimum": 0, "maximum": 3, "position": 1 },
                    "note": { "type": "string", "maxLength": 50, "position": 2 },
                },
                "additionalProperties": false,
            });
            if !fields.is_empty() {
                schema
                    .insert(
                        "moderatorAbilities".to_string(),
                        platform_value!({ "changeFields": fields }),
                    )
                    .expect("expected to set the fields");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "report",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Added, removed or swapped, the list is fixed: a field only moderators write starts
        // absent on every document, and one its owner could set would no longer be theirs.
        for (old_fields, new_fields, from, to) in [
            (&[][..], &["status"][..], "none", "status"),
            (&["status"][..], &[][..], "status", "none"),
            (
                &["status"][..],
                &["note", "status"][..],
                "status",
                "note, status",
            ),
            (&["status"][..], &["note"][..], "status", "note"),
        ] {
            let result = make_document_type(old_fields)
                .as_ref()
                .validate_update(make_document_type(new_fields).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            let expected = format!(
                "document type can not change which fields only its moderators write: changing from {from} to {to}"
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(&["status"])
            .as_ref()
            .validate_update(
                make_document_type(&["status"]).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_the_time_to_live_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        let make_document_type = |ttl: Option<u32>| {
            let mut schema = platform_value!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "required": ["$createdAt"],
                "additionalProperties": false,
            });
            if let Some(seconds) = ttl {
                schema
                    .insert("ttl".to_string(), seconds.into())
                    .expect("expected to set the time to live");
            }
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "note",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };

        // Stored documents carry the expiry they were written and paid with: adding,
        // removing, lengthening and shortening the time to live are all refused, before the
        // schema compatibility differ, which only freezes the key's text.
        for (old_ttl, new_ttl, from, to) in [
            (Some(86400), Some(172800), "86400 seconds", "172800 seconds"),
            (Some(86400), Some(3600), "86400 seconds", "3600 seconds"),
            (Some(86400), None, "86400 seconds", "no time to live"),
            (None, Some(86400), "no time to live", "86400 seconds"),
        ] {
            let result = make_document_type(old_ttl)
                .as_ref()
                .validate_update(make_document_type(new_ttl).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            let expected = format!(
                "document type can not change the time to live of its documents: changing from {from} to {to}"
            );
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(Some(86400))
            .as_ref()
            .validate_update(
                make_document_type(Some(86400)).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_return_invalid_result_when_only_a_consume_deleting_documents_is_changed() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        let make_document_type = |can_be_deleted: Value| {
            let mut schema = platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
            });
            schema
                .insert("canBeDeleted".to_string(), can_be_deleted)
                .expect("expected to set canBeDeleted");
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "commitment",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };
        let only_when_consumed = || Value::Text("onlyWhenConsumed".to_string());

        // Setting it would let documents leave state under the permanent references made to
        // a type that promised they never would, and removing it would strand the references
        // that consume them: both refused, with the error naming the change. From or to
        // `true` the owner's flag changes too, which the check of the owner's delete reports
        // first.
        for (old, new, expected) in [
            (
                Value::Bool(false),
                only_when_consumed(),
                "document type can not change whether only a consume deletes its documents: changing canBeDeleted from false to \"onlyWhenConsumed\"",
            ),
            (
                only_when_consumed(),
                Value::Bool(false),
                "document type can not change whether only a consume deletes its documents: changing canBeDeleted from \"onlyWhenConsumed\" to false",
            ),
            (
                Value::Bool(true),
                only_when_consumed(),
                "document type can not change whether its documents can be deleted: changing from true to false",
            ),
            (
                only_when_consumed(),
                Value::Bool(true),
                "document type can not change whether its documents can be deleted: changing from false to true",
            ),
        ] {
            let result = make_document_type(old)
                .as_ref()
                .validate_update(make_document_type(new).as_ref(), 2, platform_version)
                .expect("validate_update should not error");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message() == expected,
                "expected {expected:?}, got {:?}",
                result.errors
            );
        }

        // Unchanged, it passes.
        let result = make_document_type(only_when_consumed())
            .as_ref()
            .validate_update(
                make_document_type(only_when_consumed()).as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_refuse_adding_a_generated_property_over_existing_params() {
        let platform_version = PlatformVersion::latest();
        let data_contract_id = Identifier::random();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        let make_document_type = |properties: Value| {
            let schema = platform_value!({
                "type": "object",
                "properties": properties,
                "additionalProperties": false,
            });
            DocumentType::try_from_schema(
                data_contract_id,
                1,
                config.version(),
                "handle",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("document type should parse")
        };
        let string = |position: u64| platform_value!({ "type": "string", "maxLength": 32, "position": position });
        let generated = |position: u64, source: &str| {
            platform_value!({
                "type": "string", "maxLength": 32, "position": position,
                "generatedFrom": {
                    "function": "sys.stringTransformations.homographSafeASCII",
                    "params": [source]
                }
            })
        };
        let old = make_document_type(platform_value!({ "label": string(0) }));

        // Stored handles hold a label but would never hold the generated value
        let result = old
            .as_ref()
            .validate_update(
                make_document_type(platform_value!({
                    "label": string(0),
                    "normalizedLabel": generated(1, "label")
                }))
                .as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message().contains(
                    "can not add property \"normalizedLabel\" generated from existing properties (label)"
                )
        );

        // A new property generated from a new param holds for every stored document:
        // neither is there
        let result = old
            .as_ref()
            .validate_update(
                make_document_type(platform_value!({
                    "label": string(0),
                    "nickname": string(1),
                    "normalizedNickname": generated(2, "nickname")
                }))
                .as_ref(),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_removing_an_immutable_property() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a", "b"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["a"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message()
                    == "document type can not remove immutable property 'b': the properties immutable lists without a condition may only grow"
        );
    }

    // Adding an entry narrows what future replaces may touch and invalidates
    // no stored document. This runs the whole v1 pipeline, so it also pins
    // that the compatibility differ ignores the top-level `immutable` key
    // instead of hard-erroring on a keyword it has no rule for.
    #[test]
    fn should_accept_adding_an_immutable_property() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["a", "c"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "growing the immutable list must be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_accept_an_unchanged_immutable_list() {
        let platform_version = PlatformVersion::latest();

        let old = doc_type_with_immutable(platform_value!(["a", "b"]), platform_version);
        let new = doc_type_with_immutable(platform_value!(["b", "a"]), platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "an unchanged (reordered) immutable list must be accepted, got {:?}",
            result.errors
        );
    }

    /// `doc_type_with_immutable` with a conditional entry freezing each
    /// `(property, when)` of `conditions` after the plain `immutable` names.
    fn doc_type_with_conditions(
        immutable: &[&str],
        conditions: &[(&str, Value)],
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let entries = immutable
            .iter()
            .map(|property| Value::Text(property.to_string()))
            .chain(conditions.iter().map(|(property, when)| {
                platform_value!({ "property": property.to_string(), "when": when.clone() })
            }))
            .collect();
        doc_type_with_immutable(Value::Array(entries), platform_version)
    }

    fn validate_conditions_update(
        old: DocumentType,
        new: DocumentType,
    ) -> SimpleConsensusValidationResult {
        old.as_ref()
            .validate_update(new.as_ref(), 2, PlatformVersion::latest())
            .expect("validate_update should not error")
    }

    fn present(path: &str) -> Value {
        platform_value!({ "present": path })
    }

    fn update_error(result: &SimpleConsensusValidationResult) -> String {
        match result.errors.as_slice() {
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))] => {
                e.additional_message().to_string()
            }
            other => panic!("expected one DocumentTypeUpdateError, got {other:?}"),
        }
    }

    // Each runs the whole v1 pipeline, so the accepted ones also pin that the
    // compatibility differ ignores the conditional entries of `immutable`
    // instead of hard-erroring on a shape it has no rule for.
    #[test]
    fn should_accept_keeping_or_adding_a_condition() {
        let platform_version = PlatformVersion::latest();
        let old =
            || doc_type_with_conditions(&["a"], &[("b", present("$old.b"))], platform_version);

        let result = validate_conditions_update(old(), old());
        assert!(
            result.is_valid(),
            "an unchanged condition: {:?}",
            result.errors
        );

        let result = validate_conditions_update(
            old(),
            doc_type_with_conditions(
                &["a"],
                &[("b", present("$old.b")), ("c", present("$old.c"))],
                platform_version,
            ),
        );
        assert!(
            result.is_valid(),
            "a property newly frozen by a condition: {:?}",
            result.errors
        );
    }

    /// Frozen whatever the condition says is tighter than any condition.
    #[test]
    fn should_accept_dropping_a_condition_to_list_the_property_without_one() {
        let platform_version = PlatformVersion::latest();
        let result = validate_conditions_update(
            doc_type_with_conditions(&["a"], &[("b", present("$old.b"))], platform_version),
            doc_type_with_conditions(&["a", "b"], &[], platform_version),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// Whether one condition holds wherever another does can not be told in
    /// general, so a changed condition is refused, tighter or not.
    #[test]
    fn should_reject_changing_a_condition() {
        let platform_version = PlatformVersion::latest();
        let result = validate_conditions_update(
            doc_type_with_conditions(&[], &[("b", present("$old.b"))], platform_version),
            doc_type_with_conditions(&[], &[("b", present("$old.c"))], platform_version),
        );
        assert_eq!(
            update_error(&result),
            "document type can not change the condition under which immutable property 'b' is \
             frozen: it may only be listed without one"
        );
    }

    #[test]
    fn should_reject_removing_a_property_listed_with_a_condition() {
        let platform_version = PlatformVersion::latest();
        let result = validate_conditions_update(
            doc_type_with_conditions(&["a"], &[("b", present("$old.b"))], platform_version),
            doc_type_with_conditions(&["a"], &[], platform_version),
        );
        assert_eq!(
            update_error(&result),
            "document type can not remove immutable property 'b': a property listed with a \
             condition may only be listed without one"
        );
    }

    /// A property frozen at creation may not gain a condition, which would
    /// free it while the condition does not hold; the property left the list
    /// of those without one, which may only grow.
    #[test]
    fn should_reject_giving_an_immutable_property_a_condition() {
        let platform_version = PlatformVersion::latest();
        let result = validate_conditions_update(
            doc_type_with_conditions(&["a", "b"], &[], platform_version),
            doc_type_with_conditions(&["a"], &[("b", present("$old.b"))], platform_version),
        );
        assert_eq!(
            update_error(&result),
            "document type can not remove immutable property 'b': the properties immutable lists \
             without a condition may only grow"
        );
    }

    /// A document type with a string `a` and a required integer `n`, whose
    /// schema also carries every `(key, value)` of `extra`. Its contract has a
    /// token at position 0 and declares moderation, so any document type
    /// keyword may be set.
    fn doc_type_with_keywords(extra: Value, platform_version: &PlatformVersion) -> DocumentType {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "n": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000_u64},
            },
            "required": ["n"],
            "additionalProperties": false,
        });
        for (key, value) in extra.into_btree_string_map().expect("extra is a map") {
            schema
                .insert(key, value)
                .expect("expected to set the keyword");
        }
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            }));
        let token_configurations = BTreeMap::from([(
            0,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        )]);
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &token_configurations,
            &config,
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // The fees of a published document type do not change yet, although a transition names
    // the fee it agrees to pay.
    #[test]
    fn should_reject_adding_changing_or_removing_action_fees() {
        let platform_version = PlatformVersion::latest();
        let fees = |action_fees: Value| {
            doc_type_with_keywords(
                platform_value!({ "actionFees": action_fees }),
                platform_version,
            )
        };
        let free = doc_type_with_keywords(platform_value!({}), platform_version);
        let priced = fees(platform_value!({"create": {"owner": 10_u64}}));
        let repriced = fees(platform_value!({"create": {"owner": 11_u64}}));
        let fixed = fees(platform_value!({"pricing": "fixed", "create": {"owner": 10_u64}}));

        for (old, new, change) in [
            (&free, &priced, "add"),
            (&priced, &free, "remove"),
            (&priced, &repriced, "change"),
            (&priced, &fixed, "change"),
        ] {
            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("expected the update to be judged");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains(&format!("can not {change} its action fees"))
            );
        }
    }

    #[test]
    fn should_accept_unchanged_action_fees() {
        let platform_version = PlatformVersion::latest();
        let priced = doc_type_with_keywords(
            platform_value!({"actionFees": {"create": {"owner": 10_u64, "moderators": 3_u64}}}),
            platform_version,
        );
        let result = priced
            .as_ref()
            .validate_update(priced.as_ref(), 2, platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    // Token costs are fixed when the document type is published, as its action fees are,
    // and a change is refused with a consensus error before the schema compatibility
    // differ runs.
    #[test]
    fn should_return_invalid_result_when_token_costs_are_changed() {
        let platform_version = PlatformVersion::latest();
        // Transferable and tradeable, so that every action may carry a cost
        let costing = |token_cost: Vec<(&str, Value)>| {
            let token_cost = Value::Map(
                token_cost
                    .into_iter()
                    .map(|(action, cost)| (Value::Text(action.to_string()), cost))
                    .collect(),
            );
            doc_type_with_keywords(
                platform_value!({"transferable": 1_u64, "tradeMode": 1_u64, "tokenCost": token_cost}),
                platform_version,
            )
        };
        let cost = |amount: u64| platform_value!({"tokenPosition": 0_u64, "amount": amount});
        let assert_refused = |old: &DocumentType, new: &DocumentType, expected: &str| {
            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("expected the update to be judged");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains(expected),
                "{expected}: {:?}",
                result.errors
            );
        };

        let free = costing(vec![]);
        for action in [
            "create",
            "replace",
            "delete",
            "transfer",
            "update_price",
            "purchase",
        ] {
            let priced = costing(vec![(action, cost(1))]);
            let repriced = costing(vec![(action, cost(2))]);
            assert_refused(
                &free,
                &priced,
                &format!("can not add the token cost of its {action} action"),
            );
            assert_refused(
                &priced,
                &free,
                &format!("can not remove the token cost of its {action} action"),
            );
            assert_refused(
                &priced,
                &repriced,
                &format!("can not change the token cost of its {action} action"),
            );

            let result = priced
                .as_ref()
                .validate_update(priced.as_ref(), 2, platform_version)
                .expect("expected the update to be judged");
            assert!(result.is_valid(), "{action}: {:?}", result.errors);
        }

        // Every part of a cost is fixed with it, not only the amount
        let create_1 = costing(vec![("create", cost(1))]);
        for changed in [
            platform_value!({"tokenPosition": 0_u64, "amount": 1_u64, "effect": 1_u64}),
            platform_value!({"tokenPosition": 0_u64, "amount": 1_u64, "gasFeesPaidBy": 1_u64}),
            platform_value!({"tokenPosition": 0_u64, "amount": 1_u64, "optional": true}),
        ] {
            assert_refused(
                &create_1,
                &costing(vec![("create", changed)]),
                "can not change the token cost of its create action",
            );
        }
    }

    // An edit that leaves every parsed value as it was, such as writing out a default or
    // switching to the averageable shorthand, still changes the schema text. None of these
    // keywords has a rule in the shared rule set; the differ freezes each, so the edit is an
    // incompatible schema change, as the same edit to `documentsMutable` is.
    #[test]
    fn should_refuse_a_schema_edit_that_leaves_the_parsed_document_type_unchanged() {
        let platform_version = PlatformVersion::latest();
        let cost = platform_value!({"tokenPosition": 0_u64, "amount": 1_u64});
        let cost_written_out = platform_value!({
            "tokenPosition": 0_u64,
            "amount": 1_u64,
            "effect": 0_u64,
            "gasFeesPaidBy": 0_u64,
            "optional": false,
        });

        for (old_keywords, new_keywords, expected_path) in [
            (
                platform_value!({"tokenCost": {"create": cost.clone()}}),
                platform_value!({"tokenCost": {"create": cost_written_out}}),
                "/tokenCost/create/effect",
            ),
            (
                platform_value!({"actionFees": {"create": {"owner": 10_u64}}}),
                platform_value!({"actionFees": {"create": {"owner": 10_u64, "moderators": 0_u64}}}),
                "/actionFees/create/moderators",
            ),
            (
                platform_value!({}),
                platform_value!({"keepsTransferHistory": false}),
                "/keepsTransferHistory",
            ),
            (
                platform_value!({}),
                platform_value!({"keepsPurchaseHistory": false}),
                "/keepsPurchaseHistory",
            ),
            (
                platform_value!({}),
                platform_value!({"keepsPricingHistory": false}),
                "/keepsPricingHistory",
            ),
            (
                platform_value!({}),
                platform_value!({"documentsCountable": false}),
                "/documentsCountable",
            ),
            (
                platform_value!({}),
                platform_value!({"rangeCountable": false}),
                "/rangeCountable",
            ),
            (
                platform_value!({}),
                platform_value!({"indexOnly": false}),
                "/indexOnly",
            ),
            (
                platform_value!({}),
                platform_value!({"moderatorAbilities": {"delete": false}}),
                "/moderatorAbilities",
            ),
            (
                platform_value!({"documentsCountable": true, "documentsSummable": "n"}),
                platform_value!({"documentsAverageable": "n"}),
                "/documentsAverageable",
            ),
            (
                platform_value!({
                    "documentsAverageable": "n",
                    "rangeCountable": true,
                    "rangeSummable": true,
                }),
                platform_value!({"documentsAverageable": "n", "rangeAverageable": true}),
                "/rangeAverageable",
            ),
            (
                platform_value!({}),
                platform_value!({"minProperties": 0_u64}),
                "/minProperties",
            ),
            (
                platform_value!({"maxProperties": 2_u64}),
                platform_value!({"maxProperties": 3_u64}),
                "/maxProperties",
            ),
        ] {
            let old = doc_type_with_keywords(old_keywords.clone(), platform_version);
            let new = doc_type_with_keywords(new_keywords.clone(), platform_version);
            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .unwrap_or_else(|error| {
                    panic!("{old_keywords:?} -> {new_keywords:?} must be judged, got {error:?}")
                });
            assert!(
                !result.errors.is_empty()
                    && result.errors.iter().all(|error| matches!(
                        error,
                        ConsensusError::BasicError(
                            BasicError::IncompatibleDocumentTypeSchemaError(_)
                        )
                    )),
                "{old_keywords:?} -> {new_keywords:?}: {:?}",
                result.errors
            );
            assert!(
                result.errors.iter().any(|error| matches!(
                    error,
                    ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))
                        if e.property_path() == expected_path
                )),
                "{old_keywords:?} -> {new_keywords:?}: {:?}",
                result.errors
            );
        }
    }

    // The v0 regression this generation fixes: the outcome of adding an
    // index must not depend on where its name sorts relative to the
    // document type's existing indexes. Under v0, adding "i" on [a] was
    // rejected with an opaque "Invalid path" (level renumbering) while the
    // semantically identical "z" on [a] passed the tree comparison and
    // hard-errored later in schema compatibility. Under v1 both get the
    // same clean rejection naming the added index.
    #[test]
    fn should_reject_added_index_identically_regardless_of_name_sort_order() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new_early_name = doc_type_with_indices(
            platform_value!([
                {"name": "i", "properties": [{"a": "asc"}]},
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let new_late_name = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "z", "properties": [{"a": "asc"}]},
            ]),
            platform_version,
        );

        let early_result = old
            .as_ref()
            .validate_update(new_early_name.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            early_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "added index 'i'"
        );

        let late_result = old
            .as_ref()
            .validate_update(new_late_name.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            late_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "added index 'z'"
        );
    }

    // Renaming an index leaves the `IndexLevel` tree unchanged (index names
    // are not part of it), so under v0 a rename either slipped through to a
    // schema-compatibility hard error or — when it shifted the name-order
    // level numbering — was rejected as "Invalid path". Under v1 it is a
    // clean, deterministic rejection.
    #[test]
    fn should_reject_renamed_index_with_clean_error() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        // "j" renamed to "zz" — this also shifts the v0 level numbering
        // because "zz" sorts after "k" while "j" sorted before it.
        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "zz", "properties": [{"c": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "removed index 'j'"
        );
    }

    #[test]
    fn should_reject_removed_index() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "removed index 'j'"
        );
    }

    #[test]
    fn should_reject_index_with_added_property() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}, {"a": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "changed index 'j'"
        );
    }

    // Flipping `unique` leaves the v0 `IndexLevel` subset comparison
    // blind (it never compared terminator info), so v0 let it through to
    // the schema-compatibility hard error. v1 rejects it cleanly.
    #[test]
    fn should_reject_index_with_changed_unique_flag() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}], "unique": true},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "changed index 'j'"
        );
    }

    // Reordering the `indices` array without changing the definition set is
    // a semantic no-op (indices are keyed by name), so the name-keyed
    // comparison passes — and the schema-compatibility check must not trip
    // over the surviving `/indices` JSON diff. Under protocol v13 that diff
    // hit the unsupported-keyword hard error (an internal error, not a
    // consensus-invalid result); at v14 `validate_schema_compatibility` v1
    // strips `indices` before diffing and the update validates cleanly.
    #[test]
    fn should_pass_when_indices_are_reordered_without_changes() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);

        let new = doc_type_with_indices(
            platform_value!([
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "j", "properties": [{"c": "asc"}]},
            ]),
            platform_version,
        );

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "a reorder-only indices update should be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_pass_when_indices_are_unchanged() {
        let platform_version = PlatformVersion::latest();

        let old = old_doc_type(platform_version);
        let new = old_doc_type(platform_version);

        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");

        assert!(
            result.is_valid(),
            "unchanged document type should be accepted, got {:?}",
            result.errors
        );
    }

    // Ranked aggregate indexes (protocol v14 grammar) are covered by the
    // same name-keyed definition comparison as every other index flag:
    // toggling a ranking axis after creation changes the on-disk tree
    // variant, so it must be rejected. The ranking axes are index-level,
    // so `validate_config` — which covers the *doctype*-level count / sum
    // flags — is deliberately not where they are enforced. These tests
    // exercise the PUBLIC dispatcher so that routing is pinned, not just
    // the helper in isolation; they live here rather than in v0 because
    // the ranked grammar only exists at protocol v14, where
    // validate_update dispatches to v1.
    mod validate_update_ranked_indices {
        use super::*;

        /// `review` doctype, one averageable index over `restaurantId`, with
        /// `rankedAverageable` set to the supplied value.
        fn document_type_with_ranked_index(
            ranked_averageable: bool,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    // 32 rather than the generic 63-character index limit:
                    // an index declaring a ranking axis bounds its group key
                    // more tightly (59 characters on the Avg axis), and both
                    // halves of these tests have to build the same doctype
                    // shape with only `rankedAverageable` differing.
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 0,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 1,
                    },
                },
                "required": ["restaurantId", "grade"],
                "additionalProperties": false,
                "indices": [{
                    "name": "byRestaurant",
                    "properties": [{ "restaurantId": "asc" }],
                    "averageable": "grade",
                    "rangeAverageable": true,
                    "rankedAverageable": ranked_averageable,
                }],
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "review",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        #[test]
        fn should_return_invalid_result_when_ranked_averageable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(false, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == "changed index 'byRestaurant'"
            );
        }

        #[test]
        fn should_pass_when_ranked_averageable_is_unchanged() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(true, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert!(
                result.is_valid(),
                "an unchanged ranked index must not be rejected, got {:?}",
                result.errors
            );
        }
    }

    // ================================================================
    //  Required-field updates (`requiredSince`)
    // ================================================================

    mod required_fields_update {
        use super::*;

        fn doc_type_with(
            properties: Value,
            required: Value,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false,
            });
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentType::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        fn old_doc_type(platform_version: &PlatformVersion) -> DocumentType {
            doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            )
        }

        #[test]
        fn should_allow_adding_new_required_property_with_correct_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert!(
                result.is_valid(),
                "a new required property annotated with the version this \
                 update creates must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_reject_new_required_property_with_retroactive_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            // Contract moving to version 3, but the annotation claims 2:
            // documents stamped 2 would misparse
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 3, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details().contains("must carry requiredSince 3")
            );
        }

        #[test]
        fn should_reject_new_required_property_without_required_since() {
            let platform_version = PlatformVersion::latest();

            let old = old_doc_type(platform_version);
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details().contains("must carry requiredSince 2")
            );
        }

        #[test]
        fn should_reject_promoting_existing_property_to_required() {
            let platform_version = PlatformVersion::latest();

            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details() == "existing property 'b' cannot become required"
            );
        }

        #[test]
        fn should_reject_removing_required_field() {
            let platform_version = PlatformVersion::latest();

            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                }),
                platform_value!(["a"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
                )] if e.details() == "removed required field 'b'"
            );
        }

        #[test]
        fn should_reject_mutating_required_since_on_existing_property() {
            let platform_version = PlatformVersion::latest();

            // The property was added as required at version 2; a later
            // update must not move the annotation. This is caught by the
            // compatibility differ's frozen `requiredSince` rule.
            let old = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 2},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );
            let new = doc_type_with(
                platform_value!({
                    "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 60_u32, "requiredSince": 3},
                }),
                platform_value!(["a", "b"]),
                platform_version,
            );

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), 3, platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDocumentTypeSchemaError(e)
                )] if e.operation() == "replace" && e.property_path() == "/properties/b/requiredSince"
            );
        }
    }

    // ================================================================
    //  Typed array element encoding
    // ================================================================

    mod typed_array_element_encoding {
        use super::*;

        /// A document type whose one property is a typed array with the
        /// given `items` and `maxItems`.
        fn doc_type_with_list(
            items: Value,
            max_items: u16,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "list": {
                        "type": "array",
                        "maxItems": max_items,
                        "items": items,
                        "position": 0
                    },
                },
                "additionalProperties": false,
            });
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentType::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        /// The schema compatibility rules allow each of these changes, but
        /// each changes how an element is written, so the elements already
        /// stored would be misread.
        #[test]
        fn should_reject_an_update_that_changes_how_typed_array_elements_are_encoded() {
            let platform_version = PlatformVersion::latest();

            for (old_items, new_items, old_encoding, new_encoding) in [
                // Raising the maximum across a width boundary widens the
                // element from one byte to two
                (
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 1000 }),
                    "u8",
                    "u16",
                ),
                // So does adding an enum value past what a byte holds
                (
                    platform_value!({ "type": "integer", "enum": [1, 2, 3] }),
                    platform_value!({ "type": "integer", "enum": [1, 2, 3, 300] }),
                    "u8",
                    "u16",
                ),
                // A byte array element whose size stops being pinned gains a
                // length prefix
                (
                    platform_value!({
                        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20
                    }),
                    platform_value!({
                        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 32
                    }),
                    "a fixed 20-byte array",
                    "a length-prefixed byte array",
                ),
            ] {
                let old = doc_type_with_list(old_items.clone(), 8, platform_version);
                let new = doc_type_with_list(new_items.clone(), 8, platform_version);

                let result = old
                    .as_ref()
                    .validate_update(new.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                let expected = format!(
                    "document type can not change the element encoding of typed array property \
                     'list': its elements are stored as {old_encoding} and would be read as \
                     {new_encoding}"
                );
                assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                        if e.additional_message() == expected,
                    "{old_items:?} -> {new_items:?}: {:?}",
                    result.errors
                );
            }
        }

        /// Longer strings, more elements, and a raised maximum that stays
        /// within the element's width leave every stored element readable.
        #[test]
        fn should_accept_an_update_that_keeps_how_typed_array_elements_are_encoded() {
            let platform_version = PlatformVersion::latest();

            for (old_items, old_max_items, new_items, new_max_items) in [
                (
                    platform_value!({ "type": "string", "maxLength": 20 }),
                    8,
                    platform_value!({ "type": "string", "maxLength": 40 }),
                    8,
                ),
                (
                    platform_value!({
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    }),
                    8,
                    platform_value!({
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    }),
                    64,
                ),
                (
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 100 }),
                    8,
                    platform_value!({ "type": "integer", "minimum": 0, "maximum": 200 }),
                    8,
                ),
            ] {
                let old = doc_type_with_list(old_items.clone(), old_max_items, platform_version);
                let new = doc_type_with_list(new_items.clone(), new_max_items, platform_version);

                let result = old
                    .as_ref()
                    .validate_update(new.as_ref(), 2, platform_version)
                    .expect("validate_update should not error");

                assert!(
                    result.is_valid(),
                    "{old_items:?} ({old_max_items}) -> {new_items:?} ({new_max_items}): {:?}",
                    result.errors
                );
            }
        }
    }

    // ================================================================
    //  Integer encoding (width and signedness)
    // ================================================================

    mod integer_encoding_update {
        use super::*;
        use crate::data_contract::config::v1::DataContractConfigSettersV1;
        use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use crate::data_contract::document_type::DocumentPropertyType;
        use crate::validation::SimpleConsensusValidationResult;
        use std::io::BufReader;

        fn doc_type_with(
            properties: Value,
            sized_integer_types: bool,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": properties,
                "additionalProperties": false,
            });
            let mut config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            config.set_sized_integer_types_enabled(sized_integer_types);
            DocumentType::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        /// A document type whose one property, `score`, has the given schema.
        fn doc_type_with_score(score: Value, platform_version: &PlatformVersion) -> DocumentType {
            doc_type_with(platform_value!({ "score": score }), true, platform_version)
        }

        fn validate_update(
            old: &DocumentType,
            new: &DocumentType,
            platform_version: &PlatformVersion,
        ) -> SimpleConsensusValidationResult {
            old.as_ref()
                .validate_update(new.as_ref(), 2, platform_version)
                .expect("validate_update should not error")
        }

        fn assert_rejected(
            result: SimpleConsensusValidationResult,
            path: &str,
            old: &str,
            new: &str,
        ) {
            let expected =
                format!("'{path}': its values are stored as {old} and would be read as {new}");
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                    if e.additional_message().contains(&expected),
                "expected the integer encoding change {old} -> {new} of '{path}' to be refused"
            );
        }

        #[test]
        fn should_not_read_a_stored_u8_back_as_the_u16_a_raised_maximum_gives() {
            // Why the type is held: the bounds choose it, and a value stored
            // at the old width does not read back at the new one.
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );
            let old_type = &old.flattened_properties()["score"].property_type;
            let new_type = &new.flattened_properties()["score"].property_type;
            assert_eq!(old_type, &DocumentPropertyType::U8);
            assert_eq!(new_type, &DocumentPropertyType::U16);

            let stored = old_type
                .encode_value_ref_with_size(&Value::U8(7), true)
                .expect("should encode a u8");
            assert_eq!(stored, vec![7]);

            let read_back =
                new_type.read_optionally_from(&mut BufReader::new(stored.as_slice()), true);
            assert!(
                read_back.is_err(),
                "a stored u8 must not read back as a u16, got {read_back:?}"
            );
        }

        #[test]
        fn should_reject_raising_maximum_past_the_width_of_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_report_a_width_change_of_an_integer_range_source_as_an_integer_encoding_change() {
            // The grid is the same declaration on both sides; only the
            // source's width moves, so the refusal names the property's
            // encoding, exactly as for a plain index on it.
            let platform_version = PlatformVersion::latest();
            let doc_type = |maximum: u64| {
                let schema = platform_value!({
                    "type": "object",
                    "properties": {
                        "score": {"type": "integer", "minimum": 0, "maximum": maximum, "position": 0}
                    },
                    "required": ["score"],
                    "indices": [{
                        "name": "byBand",
                        "properties": [{"score": "asc"}],
                        "integerRange": {"on": "score", "range": 10u64, "step": 10u64}
                    }],
                    "additionalProperties": false,
                });
                let mut config = DataContractConfig::default_for_version(platform_version)
                    .expect("should create a default config");
                config.set_sized_integer_types_enabled(true);
                DocumentType::try_from_schema(
                    Identifier::new([1; 32]),
                    1,
                    config.version(),
                    "test",
                    schema,
                    None,
                    &BTreeMap::new(),
                    &config,
                    false,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("failed to create document type")
            };

            assert_rejected(
                validate_update(&doc_type(100), &doc_type(1000), platform_version),
                "score",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_lowering_minimum_below_zero() {
            // The same width with the other signedness: a stored u64 above
            // i64::MAX would read back negative
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": -1, "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u64",
                "i64",
            );
        }

        #[test]
        fn should_reject_removing_the_bounds() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "i64",
            );
        }

        #[test]
        fn should_reject_adding_an_enum_value_past_the_width_of_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "enum": [1, 2, 3], "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "enum": [1, 2, 3, 300], "position": 0}),
                platform_version,
            );

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_a_width_change_of_a_nested_integer_property() {
            let platform_version = PlatformVersion::latest();
            let stats = |maximum: u32| {
                platform_value!({
                    "stats": {
                        "type": "object",
                        "position": 0,
                        "properties": {
                            "level": {"type": "integer", "minimum": 0, "maximum": maximum, "position": 0},
                        },
                        "additionalProperties": false,
                    },
                })
            };
            let old = doc_type_with(stats(100), true, platform_version);
            let new = doc_type_with(stats(1000), true, platform_version);

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "stats.level",
                "u8",
                "u16",
            );
        }

        #[test]
        fn should_reject_turning_sized_integer_types_on() {
            // Off, every integer is an i64; on, the bounds make this one a u8.
            // The contract config check only refuses turning them off.
            let platform_version = PlatformVersion::latest();
            let properties = platform_value!({
                "score": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0},
            });
            let old = doc_type_with(properties.clone(), false, platform_version);
            let new = doc_type_with(properties, true, platform_version);

            assert_rejected(
                validate_update(&old, &new, platform_version),
                "score",
                "i64",
                "u8",
            );
        }

        #[test]
        fn should_accept_a_bound_change_that_keeps_the_type() {
            let platform_version = PlatformVersion::latest();
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 200, "position": 0}),
                platform_version,
            );

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "a u8 that stays a u8 must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_accept_raising_maximum_without_sized_integer_types() {
            // Without sized integer types every integer is an i64 whatever
            // its bounds, so raising one changes nothing stored
            let platform_version = PlatformVersion::latest();
            let score = |maximum: u32| {
                platform_value!({
                    "score": {"type": "integer", "minimum": 0, "maximum": maximum, "position": 0},
                })
            };
            let old = doc_type_with(score(100), false, platform_version);
            let new = doc_type_with(score(1000), false, platform_version);

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "an i64 that stays an i64 must be accepted, got {:?}",
                result.errors
            );
        }

        #[test]
        fn should_still_accept_a_width_change_at_protocol_version_13() {
            // validate_update v0 is frozen for replay of protocol versions up
            // to 13, which let the width move
            let platform_version =
                PlatformVersion::get(13).expect("protocol version 13 must exist");
            let old = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 100, "position": 0}),
                platform_version,
            );
            let new = doc_type_with_score(
                platform_value!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}),
                platform_version,
            );

            let result = validate_update(&old, &new, platform_version);
            assert!(
                result.is_valid(),
                "protocol version 13 must keep accepting the width change, got {:?}",
                result.errors
            );
        }
    }
}
