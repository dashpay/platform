//! The values of derived index properties (protocol version 14).
//!
//! An index of a stored document type may name a value of the document a reference of the
//! type points at, `"<reference property>.<field>"` (such as a reply's `postId.$ownerId`),
//! which the document never stores. Before Drive keys a document of such a type, to write its
//! index entries or to remove them, it reads each referenced document and puts the values into
//! the document's properties under the derived properties' own names, where
//! `get_raw_for_document_type` finds them. No property name holds a `.`, so they never collide
//! with a stored property, and the document serialization writes only the type's properties,
//! so they are never stored.
//!
//! The parser admits a derived property only where the value can not change once the entry is
//! written: the reference is fixed once written, and the field is fixed on a referenced type
//! whose documents never leave state (`permanentDocument`) or leave it only through a
//! moderator's removal, whose record keeps the removed document's owner and the values its
//! type lists under `moderatorAbilities.deleteKeepsFields` (`moderatedDocument`, `$ownerId` or
//! a kept field only). So a later read finds the value the entry was written under.

use crate::drive::contract::moderation::types::{
    estimated_document_removal_kept_fields_size, estimated_document_removal_value_size,
};
use crate::drive::contract::paths::contract_document_type_removals_path;
use crate::drive::document::cost::value_of;
use crate::drive::document::paths::{
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path,
};
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::grove_operations::{DirectQueryType, QueryType};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{kept_value_at, ContractDocumentRemoval};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{
    DerivedIndexField, DerivedIndexProperty, DocumentReferenceKind, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg, TreeType};
use std::collections::BTreeMap;

impl Drive {
    /// `document_and_contract_info` with its document completed by the value of every derived
    /// index property of its type (see [`Self::derived_index_values`]), ready to be keyed. A
    /// borrowed document is copied to be completed. A type declaring none, and a worst-case
    /// size, pass through unchanged, which is every write before protocol version 14.
    pub(crate) fn with_derived_index_values<'a>(
        &self,
        document_and_contract_info: DocumentAndContractInfo<'a>,
        estimate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAndContractInfo<'a>, Error> {
        let DocumentAndContractInfo {
            owned_document_info,
            contract,
            document_type,
        } = document_and_contract_info;
        let document_info = self.document_info_with_derived_index_values(
            owned_document_info.document_info,
            contract,
            document_type,
            estimate,
            transaction,
            drive_operations,
            platform_version,
        )?;
        Ok(DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info,
                owner_id: owned_document_info.owner_id,
            },
            contract,
            document_type,
        })
    }

    /// [`Self::with_derived_index_values`] for a document info alone.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn document_info_with_derived_index_values<'a>(
        &self,
        document_info: DocumentInfo<'a>,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        estimate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentInfo<'a>, Error> {
        if document_type.derived_index_properties().is_empty() {
            return Ok(document_info);
        }
        // A document may carry the values already: a create's, taken by its reference
        // validation from the documents it fetched
        let complete = |document: &mut Document,
                        drive_operations: &mut Vec<LowLevelDriveOperation>|
         -> Result<(), Error> {
            let values = self.derived_index_values(
                document,
                Some(document),
                contract,
                document_type,
                estimate,
                transaction,
                drive_operations,
                platform_version,
            )?;
            set_derived_index_values(document, values);
            Ok(())
        };
        Ok(match document_info {
            DocumentInfo::DocumentOwnedInfo((mut document, storage_flags)) => {
                complete(&mut document, drive_operations)?;
                DocumentInfo::DocumentOwnedInfo((document, storage_flags))
            }
            DocumentInfo::DocumentRefInfo((document, storage_flags)) => {
                let mut document = document.clone();
                complete(&mut document, drive_operations)?;
                DocumentInfo::DocumentOwnedInfo((document, storage_flags))
            }
            DocumentInfo::DocumentRefAndSerialization((document, serialization, storage_flags)) => {
                let mut document = document.clone();
                complete(&mut document, drive_operations)?;
                DocumentInfo::DocumentAndSerialization((
                    document,
                    serialization.to_vec(),
                    storage_flags,
                ))
            }
            DocumentInfo::DocumentAndSerialization((
                mut document,
                serialization,
                storage_flags,
            )) => {
                complete(&mut document, drive_operations)?;
                DocumentInfo::DocumentAndSerialization((document, serialization, storage_flags))
            }
            DocumentInfo::DocumentEstimatedAverageSize(size) => {
                // No document to read a reference from: only the cost of the reads
                self.add_derived_index_read_estimations(
                    contract,
                    document_type,
                    drive_operations,
                    platform_version,
                )?;
                DocumentInfo::DocumentEstimatedAverageSize(size)
            }
        })
    }

    /// The value of every derived index property of `document_type` for `document`, by the
    /// property's name: the field it names of the document its reference points at, read once
    /// per reference, `Value::Null` where that document has none or the reference is absent.
    ///
    /// A reference `known` points at the same document through is not read again when `known`
    /// holds every value read through it: they are taken from `known`, a completed version of
    /// the same document (the replaced document's, which the parser holds to the same
    /// references), or the document itself (a create's, carrying the values its reference
    /// validation took from the documents it fetched).
    ///
    /// A referenced document a moderator removed is read from its removal record, which keeps
    /// what a `moderatedDocument` reference may derive: its owner, and the values its type lists
    /// under `moderatorAbilities.deleteKeepsFields`, as the document held them, decoded only
    /// when a derived property reads one. When `estimate` is
    /// true nothing is read: the reads are priced by their worst case (the removal record's
    /// read too, and the hop to a current version keeping history), and every value is one of
    /// the field's type and typical size, which keys the document on the layout a real value
    /// writes.
    ///
    /// A referenced document that is neither in state nor removed on the record is corrupted
    /// state: the reference was validated when the document was written, and its target can
    /// only leave state on the record.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn derived_index_values(
        &self,
        document: &Document,
        known: Option<&Document>,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        estimate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, Value>, Error> {
        let mut by_reference: BTreeMap<&str, Vec<(&String, &DerivedIndexProperty)>> =
            BTreeMap::new();
        for (name, derived) in document_type.derived_index_properties() {
            by_reference
                .entry(derived.reference_property.as_str())
                .or_default()
                .push((name, derived));
        }
        let mut values = BTreeMap::new();
        for (reference_property, derived_properties) in by_reference {
            let referenced_id = reference_id(document, reference_property)?;
            let known = match known {
                Some(known)
                    if reference_id(known, reference_property)? == referenced_id
                        && derived_properties
                            .iter()
                            .all(|(name, _)| known.properties().contains_key(name.as_str())) =>
                {
                    Some(known)
                }
                _ => None,
            };
            if let Some(known) = known {
                for (name, _) in derived_properties {
                    let value = known
                        .properties()
                        .get(name.as_str())
                        .cloned()
                        .unwrap_or(Value::Null);
                    values.insert(name.clone(), value);
                }
                continue;
            }
            // Every derived property reads through the same reference, so of the same type and
            // kind
            let Some((_, first)) = derived_properties.first() else {
                continue;
            };
            let read = match referenced_id {
                None => ReferencedValues::None,
                Some(referenced_id) => self.read_referenced_values(
                    contract,
                    first,
                    referenced_id,
                    estimate,
                    transaction,
                    drive_operations,
                    platform_version,
                )?,
            };
            // The values a removal record keeps, decoded the first time a derived property
            // reads one
            let mut kept_values = None;
            for (name, derived) in derived_properties {
                let value = match &read {
                    ReferencedValues::None => Value::Null,
                    // A value of the field's type and typical size, so a dry run prices the
                    // entry on the layout a real value writes
                    ReferencedValues::Estimate => derived
                        .property_type
                        .as_ref()
                        .map(|property_type| {
                            let length = property_type
                                .middle_size(platform_version)
                                .map(u32::from)
                                .unwrap_or_default();
                            value_of(property_type, length, platform_version)
                        })
                        .unwrap_or(Value::Null),
                    ReferencedValues::Document(referenced) => {
                        derived.field.value_in(referenced).map_err(|error| {
                            Error::Drive(DriveError::CorruptedDriveState(format!(
                                "document {} of {} can not be read at {:?}: {error}",
                                referenced.id(),
                                derived.referenced_document_type_name,
                                derived.field
                            )))
                        })?
                    }
                    ReferencedValues::Removed(removal) => {
                        removed_document_value(contract, derived, removal, &mut kept_values)?
                    }
                };
                values.insert(name.clone(), value);
            }
        }
        Ok(values)
    }

    /// Reads the document `referenced_id` of the type `derived` reads through, or, when a
    /// moderator removed it, its removal record.
    #[allow(clippy::too_many_arguments)]
    fn read_referenced_values(
        &self,
        contract: &DataContract,
        derived: &DerivedIndexProperty,
        referenced_id: Identifier,
        estimate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ReferencedValues, Error> {
        let referenced_type_name = derived.referenced_document_type_name.as_str();
        let referenced_type = contract.document_type_for_name(referenced_type_name)?;
        if estimate {
            self.add_referenced_document_read_estimation(
                contract,
                referenced_type,
                derived.kind,
                drive_operations,
                platform_version,
            )?;
            return Ok(ReferencedValues::Estimate);
        }
        let element = if referenced_type.documents_keep_history() {
            // The `0` of a document keeping history is a reference to its current version
            let path = contract_documents_keeping_history_primary_key_path_for_document_id(
                contract.id_ref().as_bytes(),
                referenced_type_name,
                referenced_id.as_slice(),
            );
            self.grove_get(
                (&path).into(),
                &[0],
                QueryType::StatefulQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
        } else {
            let path = contract_documents_primary_key_path(
                contract.id_ref().as_bytes(),
                referenced_type_name,
            );
            self.grove_get_raw_optional(
                (&path).into(),
                referenced_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
        };
        match element {
            Some(Element::Item(bytes, _)) | Some(Element::ItemWithSumItem(bytes, _, _)) => {
                let document = Document::from_bytes(&bytes, referenced_type, platform_version)?;
                Ok(ReferencedValues::Document(Box::new(document)))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                "a referenced document is not an item",
            ))),
            None if derived.kind == DocumentReferenceKind::Moderated => {
                let removal = self.fetch_contract_document_removal_add_to_operations_v0(
                    contract.id(),
                    referenced_type_name,
                    referenced_id,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
                match removal {
                    Some(removal) if !removal.is_restored() => {
                        Ok(ReferencedValues::Removed(Box::new(removal)))
                    }
                    _ => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "document {referenced_id} of {referenced_type_name}, which a derived \
                         index property reads, is neither in state nor removed on the record"
                    )))),
                }
            }
            None => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                "document {referenced_id} of {referenced_type_name}, which a derived index \
                 property reads, is not in state, and its documents never leave it"
            )))),
        }
    }

    /// The worst-case cost of the reads [`Self::derived_index_values`] makes for a document
    /// of `document_type`, one per reference, when no document is at hand.
    fn add_derived_index_read_estimations(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut referenced_types: BTreeMap<&str, (&str, DocumentReferenceKind)> = BTreeMap::new();
        for derived in document_type.derived_index_properties().values() {
            referenced_types.insert(
                derived.reference_property.as_str(),
                (derived.referenced_document_type_name.as_str(), derived.kind),
            );
        }
        for (referenced_type_name, kind) in referenced_types.into_values() {
            let referenced_type = contract.document_type_for_name(referenced_type_name)?;
            self.add_referenced_document_read_estimation(
                contract,
                referenced_type,
                kind,
                drive_operations,
                platform_version,
            )?;
        }
        Ok(())
    }

    /// The worst-case cost of the reads [`Self::read_referenced_values`] makes for one
    /// document of `referenced_type`, reached through a reference of `kind`, without reading:
    /// the document, through the reference to its current version on a type keeping history,
    /// and, through a `moderatedDocument` reference, the removal record read when a moderator
    /// removed the document.
    fn add_referenced_document_read_estimation(
        &self,
        contract: &DataContract,
        referenced_type: DocumentTypeRef,
        kind: DocumentReferenceKind,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let document_size = referenced_type.estimated_size(platform_version)? as u32;
        if referenced_type.documents_keep_history() {
            let path = contract_documents_keeping_history_primary_key_path_for_document_id(
                contract.id_ref().as_bytes(),
                referenced_type.name(),
                &[0; 32],
            );
            // The `0` is a reference to the current version, loaded in one hop
            let in_tree_type = if referenced_type.documents_summable().is_some() {
                TreeType::SumTree
            } else {
                TreeType::NormalTree
            };
            self.grove_get(
                (&path).into(),
                &[0],
                QueryType::StatelessQuery {
                    in_tree_type,
                    query_target: QueryTargetValue(document_size),
                    estimated_reference_sizes: vec![document_size],
                },
                None,
                drive_operations,
                &platform_version.drive,
            )?;
        } else {
            let path = contract_documents_primary_key_path(
                contract.id_ref().as_bytes(),
                referenced_type.name(),
            );
            self.grove_get_raw_optional(
                (&path).into(),
                &[0; 32],
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: referenced_type.primary_key_tree_type(platform_version)?,
                    query_target: QueryTargetValue(document_size),
                },
                None,
                drive_operations,
                &platform_version.drive,
            )?;
        }
        if kind == DocumentReferenceKind::Moderated {
            let path = contract_document_type_removals_path(
                contract.id_ref().as_bytes(),
                referenced_type.name(),
            );
            self.grove_get_raw_optional(
                (&path).into(),
                &[0; 32],
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTargetValue(estimated_document_removal_value_size(
                        estimated_document_removal_kept_fields_size(
                            referenced_type,
                            platform_version,
                        )?,
                    )),
                },
                None,
                drive_operations,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}

/// What a derived index property reads from: the referenced document, the removal record of
/// a removed one, nothing (an absent reference), or, in a dry run, a value of the field's type.
enum ReferencedValues {
    None,
    Estimate,
    Document(Box<Document>),
    Removed(Box<ContractDocumentRemoval>),
}

/// The value `derived` reads from `removal`, the record of the referenced document a
/// moderator removed: the owner it keeps, or a value it keeps as the document held it,
/// `Value::Null` where the document held none, so the key is the one the document gave while
/// in state. `kept_values` is `None` until a derived property first reads a kept value, then
/// the record's values, decoded under the referenced type. A field the record does not keep
/// is refused by the parser, so reaching one is an error, never a key under a value the entry
/// was not written under.
fn removed_document_value(
    contract: &DataContract,
    derived: &DerivedIndexProperty,
    removal: &ContractDocumentRemoval,
    kept_values: &mut Option<BTreeMap<String, Value>>,
) -> Result<Value, Error> {
    let not_kept = || {
        Error::Drive(DriveError::CorruptedCodeExecution(
            "a moderatedDocument reference derives the referenced owner or a kept field only",
        ))
    };
    let path = match &derived.field {
        DerivedIndexField::OwnerId => {
            return Ok(Value::Identifier(removal.document_owner_id.to_buffer()))
        }
        DerivedIndexField::Property(path) => path,
        DerivedIndexField::CreatorId => return Err(not_kept()),
    };
    let referenced_type =
        contract.document_type_for_name(&derived.referenced_document_type_name)?;
    let kept_values = match kept_values {
        Some(kept_values) => kept_values,
        None => kept_values.insert(removal.kept_values(referenced_type).map_err(|error| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "the removal record of a document of {}, which a derived index property reads, \
                 keeps values its type can not read: {error}",
                derived.referenced_document_type_name
            )))
        })?),
    };
    match kept_value_at(
        referenced_type.moderator_deletion_kept_fields(),
        kept_values,
        path,
    ) {
        Some(value) => Ok(value.unwrap_or(Value::Null)),
        None => Err(not_kept()),
    }
}

/// The id `document` holds in `reference_property`, `None` when it holds none.
fn reference_id(
    document: &Document,
    reference_property: &str,
) -> Result<Option<Identifier>, Error> {
    match document.properties().get(reference_property) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.to_identifier().map(Some).map_err(|error| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "document {} holds a reference in {reference_property} that is not an \
                 identifier: {error}",
                document.id()
            )))
        }),
    }
}

/// Puts `values` into `document`'s properties under the derived properties' names.
pub(crate) fn set_derived_index_values(document: &mut Document, values: BTreeMap<String, Value>) {
    for (name, value) in values {
        document.properties_mut().insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use crate::config::DriveConfig;
    use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use crate::drive::Drive;
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [7; 32];
    const CREATOR: [u8; 32] = [8; 32];
    const POST: [u8; 32] = [1; 32];
    const CREATED_AT: u64 = 1_700_000_000_000;

    /// Every top-level entry of `extra` in place of the one of `schema` with its key.
    fn merged(mut schema: Value, extra: Value) -> Value {
        if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
            for (key, value) in extra_map {
                schema_map.retain(|(existing, _)| existing != &key);
                schema_map.push((key, value));
            }
        }
        schema
    }

    /// Posts nobody changes or deletes, with `post_extra`, and replies indexed by `index`,
    /// with `reply_extra`
    fn contract_with(post_extra: Value, index: Value, reply_extra: Value) -> DataContract {
        let post = merged(
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "topic": { "type": "string", "maxLength": 20, "position": 0 }
                },
                "required": ["topic"],
                "additionalProperties": false
            }),
            post_extra,
        );
        let reply = merged(
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": true,
                "indices": [{ "name": "byDerived", "properties": index }],
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "refersTo": { "type": "permanentDocument", "documentType": "post" },
                        "position": 0
                    },
                    "status": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 }
                },
                "required": ["postId"],
                "additionalProperties": false
            }),
            reply_extra,
        );
        let schemas = platform_value!({ "post": post, "reply": reply });
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned()
    }

    /// Replies filed under their post's topic
    fn contract() -> DataContract {
        contract_with(
            Value::Null,
            platform_value!([{ "postId.topic": "asc" }]),
            Value::Null,
        )
    }

    fn setup(contract: &DataContract) -> Drive {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");
        drive
    }

    fn document(id: [u8; 32], properties: BTreeMap<String, Value>) -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from(OWNER),
            properties,
            ..Default::default()
        })
    }

    fn post() -> Document {
        document(
            POST,
            BTreeMap::from([("topic".to_string(), Value::Text("rust".to_string()))]),
        )
    }

    fn reply(id: u8) -> Document {
        document(
            [id; 32],
            BTreeMap::from([("postId".to_string(), Value::Identifier(POST))]),
        )
    }

    fn add(
        drive: &Drive,
        contract: &DataContract,
        type_name: &str,
        document: &Document,
        apply: bool,
    ) {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(OWNER),
                    },
                    contract,
                    document_type: contract.document_type_for_name(type_name).expect("type"),
                },
                false,
                BlockInfo {
                    time_ms: CREATED_AT,
                    ..Default::default()
                },
                apply,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("add document");
    }

    fn replies_where(
        drive: &Drive,
        contract: &DataContract,
        where_clause: Value,
    ) -> Vec<Identifier> {
        let platform_version = PlatformVersion::latest();
        let query = DriveDocumentQuery::from_decomposed_values(
            where_clause,
            None,
            Some(10),
            None,
            false,
            None,
            contract,
            contract.document_type_for_name("reply").expect("reply"),
            &DriveConfig::default(),
            platform_version,
        )
        .expect("the query builds");
        drive
            .query_documents(query, None, false, None, None)
            .expect("the query runs")
            .documents_owned()
            .into_iter()
            .map(|document| document.id())
            .collect()
    }

    fn replies_on(drive: &Drive, contract: &DataContract, topic: &str) -> Vec<Identifier> {
        replies_where(
            drive,
            contract,
            platform_value!([["postId.topic", "==", topic]]),
        )
    }

    fn delete_reply(drive: &Drive, contract: &DataContract, id: u8, apply: bool) {
        drive
            .delete_document_for_contract(
                Identifier::from([id; 32]),
                contract,
                "reply",
                BlockInfo::default(),
                apply,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("delete document");
    }

    /// A reply is keyed by its post's topic on insert and found by it, a dry run of an insert
    /// or a delete prices the post's read without reading it, and a delete finds the entry
    /// under the topic read again.
    #[test]
    fn should_key_a_reply_by_its_posts_topic_and_price_the_read_on_a_dry_run() {
        let contract = contract();
        let drive = setup(&contract);
        add(&drive, &contract, "post", &post(), true);

        // A dry run reads nothing, so it works before the reply exists
        add(&drive, &contract, "reply", &reply(2), false);
        add(&drive, &contract, "reply", &reply(2), true);
        assert_eq!(
            replies_on(&drive, &contract, "rust"),
            vec![Identifier::from([2; 32])]
        );
        assert!(replies_on(&drive, &contract, "go").is_empty());

        delete_reply(&drive, &contract, 2, false);
        delete_reply(&drive, &contract, 2, true);
        assert!(replies_on(&drive, &contract, "rust").is_empty());
    }

    /// A document carrying its derived values, as a create does from its reference
    /// validation, is keyed by them without reading the referenced document: here there is
    /// none to read, and a read would find the state corrupted.
    #[test]
    fn should_key_a_document_by_the_derived_values_it_carries() {
        let contract = contract();
        let drive = setup(&contract);
        let mut reply = reply(2);
        reply
            .properties_mut()
            .insert("postId.topic".to_string(), Value::Text("rust".to_string()));
        add(&drive, &contract, "reply", &reply, true);
        assert_eq!(
            replies_on(&drive, &contract, "rust"),
            vec![Identifier::from([2; 32])]
        );
    }

    /// A post keeping its history is read through the reference to its current version, and a
    /// dry run prices that hop without reading.
    #[test]
    fn should_read_a_referenced_document_keeping_history() {
        let contract = contract_with(
            platform_value!({ "documentsKeepHistory": true }),
            platform_value!([{ "postId.topic": "asc" }]),
            Value::Null,
        );
        let drive = setup(&contract);
        add(&drive, &contract, "post", &post(), true);
        add(&drive, &contract, "reply", &reply(2), false);
        add(&drive, &contract, "reply", &reply(2), true);
        assert_eq!(
            replies_on(&drive, &contract, "rust"),
            vec![Identifier::from([2; 32])]
        );
        delete_reply(&drive, &contract, 2, true);
        assert!(replies_on(&drive, &contract, "rust").is_empty());
    }

    /// An update that changes a property of the reply's own in an index holding a derived
    /// value moves the entry: the old one is found under the value read again.
    #[test]
    fn should_move_an_entry_when_the_replys_own_indexed_property_changes() {
        let contract = contract_with(
            Value::Null,
            platform_value!([{ "postId.topic": "asc" }, { "status": "asc" }]),
            platform_value!({ "documentsMutable": true, "immutable": ["postId"] }),
        );
        let drive = setup(&contract);
        add(&drive, &contract, "post", &post(), true);
        let mut reply = reply(2);
        reply.set_revision(Some(1));
        reply.set("status", Value::U8(1));
        add(&drive, &contract, "reply", &reply, true);

        reply.set_revision(Some(2));
        reply.set("status", Value::U8(2));
        drive
            .update_document_for_contract(
                &reply,
                &contract,
                contract.document_type_for_name("reply").expect("reply"),
                Some(OWNER),
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("update document");
        let with_status = |status: u8| {
            replies_where(
                &drive,
                &contract,
                platform_value!([
                    ["postId.topic", "==", "rust"],
                    ["status", "==", Value::U8(status)]
                ]),
            )
        };
        assert_eq!(with_status(2), vec![Identifier::from([2; 32])]);
        assert!(with_status(1).is_empty());
    }

    /// The creator of a post that can change hands is derived, not its owner.
    #[test]
    fn should_key_a_reply_by_its_posts_creator() {
        let contract = contract_with(
            platform_value!({ "transferable": 1 }),
            platform_value!([{ "postId.$creatorId": "asc" }]),
            Value::Null,
        );
        let drive = setup(&contract);
        let mut post = post();
        post.set_revision(Some(1));
        post.set_creator_id(Some(Identifier::from(CREATOR)));
        add(&drive, &contract, "post", &post, true);
        add(&drive, &contract, "reply", &reply(2), true);
        let by_creator = |creator: [u8; 32]| {
            replies_where(
                &drive,
                &contract,
                platform_value!([["postId.$creatorId", "==", Value::Identifier(creator)]]),
            )
        };
        assert_eq!(by_creator(CREATOR), vec![Identifier::from([2; 32])]);
        assert!(by_creator(OWNER).is_empty());
    }

    /// A reply whose `ttl` runs out is deleted by the expiry cleanup, which finds its entry
    /// under the topic read again.
    #[test]
    fn should_remove_the_entry_of_a_reply_that_expires() {
        let contract = contract_with(
            Value::Null,
            platform_value!([{ "postId.topic": "asc" }]),
            platform_value!({ "ttl": 3600, "required": ["postId", "$createdAt"] }),
        );
        let drive = setup(&contract);
        add(&drive, &contract, "post", &post(), true);
        let mut reply = reply(2);
        reply.set_created_at(Some(CREATED_AT));
        add(&drive, &contract, "reply", &reply, true);
        assert_eq!(
            replies_on(&drive, &contract, "rust"),
            vec![Identifier::from([2; 32])]
        );

        let transaction = drive.grove.start_transaction();
        drive
            .remove_expired_documents(
                &BlockInfo {
                    time_ms: CREATED_AT + 3_600_000 + 1,
                    ..Default::default()
                },
                10,
                u32::MAX,
                Some(&transaction),
                PlatformVersion::latest(),
            )
            .expect("the cleanup runs");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commits");
        assert!(replies_on(&drive, &contract, "rust").is_empty());
    }
}
