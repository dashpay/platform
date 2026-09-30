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
//! moderator's removal, whose record keeps the removed document's owner
//! (`moderatedDocument`, `$ownerId` only). So a later read finds the value the entry was
//! written under.

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
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{
    DerivedIndexField, DerivedIndexProperty, DocumentReferenceKind, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
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
        let complete = |document: &mut Document,
                        drive_operations: &mut Vec<LowLevelDriveOperation>|
         -> Result<(), Error> {
            let values = self.derived_index_values(
                document,
                None,
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
    /// A reference `known` points at the same document through is not read again: its values
    /// are taken from `known`, a completed version of the same document (the replaced
    /// document's, which the parser holds to the same references).
    ///
    /// A referenced document a moderator removed is read from its removal record, which keeps
    /// its owner, the one field a `moderatedDocument` reference may derive. When `estimate` is
    /// true nothing is read: the reads are priced by their worst case, and every value is
    /// `Value::Null`.
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
            let fields: Vec<(&String, &DerivedIndexField)> = derived_properties
                .iter()
                .map(|(name, derived)| (*name, &derived.field))
                .collect();
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
            for (name, field) in fields {
                let value =
                    match &read {
                        ReferencedValues::None => Value::Null,
                        ReferencedValues::Document(referenced) => match field {
                            DerivedIndexField::OwnerId => {
                                Value::Identifier(referenced.owner_id().to_buffer())
                            }
                            DerivedIndexField::CreatorId => referenced
                                .creator_id()
                                .map(|creator_id| Value::Identifier(creator_id.to_buffer()))
                                .unwrap_or(Value::Null),
                            DerivedIndexField::Property(path) => referenced
                                .properties()
                                .get_optional_at_path(path)
                                .map_err(|error| {
                                    Error::Drive(DriveError::CorruptedDriveState(format!(
                                        "document {} of {} can not be read at {path}: {error}",
                                        referenced.id(),
                                        first.referenced_document_type_name
                                    )))
                                })?
                                .cloned()
                                .unwrap_or(Value::Null),
                        },
                        ReferencedValues::RemovedOwner(owner_id) => match field {
                            DerivedIndexField::OwnerId => Value::Identifier(owner_id.to_buffer()),
                            _ => return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                                "a moderatedDocument reference derives the referenced owner only",
                            ))),
                        },
                    };
                values.insert(name.clone(), value);
            }
        }
        Ok(values)
    }

    /// Reads the document `referenced_id` of the type `derived` reads through, or, when a
    /// moderator removed it, the owner its removal record keeps.
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
                drive_operations,
                platform_version,
            )?;
            return Ok(ReferencedValues::None);
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
                        Ok(ReferencedValues::RemovedOwner(removal.document_owner_id))
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
        let mut referenced_types: BTreeMap<&str, &str> = BTreeMap::new();
        for derived in document_type.derived_index_properties().values() {
            referenced_types.insert(
                derived.reference_property.as_str(),
                derived.referenced_document_type_name.as_str(),
            );
        }
        for referenced_type_name in referenced_types.into_values() {
            let referenced_type = contract.document_type_for_name(referenced_type_name)?;
            self.add_referenced_document_read_estimation(
                contract,
                referenced_type,
                drive_operations,
                platform_version,
            )?;
        }
        Ok(())
    }

    /// The worst-case cost of reading one document of `referenced_type`.
    fn add_referenced_document_read_estimation(
        &self,
        contract: &DataContract,
        referenced_type: DocumentTypeRef,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = contract_documents_primary_key_path(
            contract.id_ref().as_bytes(),
            referenced_type.name(),
        );
        self.grove_get_raw_optional(
            (&path).into(),
            &[0; 32],
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: referenced_type.primary_key_tree_type(platform_version)?,
                query_target: QueryTargetValue(
                    referenced_type.estimated_size(platform_version)? as u32
                ),
            },
            None,
            drive_operations,
            &platform_version.drive,
        )?;
        Ok(())
    }
}

/// What a derived index property reads from: the referenced document, the owner the removal
/// record of a removed one keeps, or nothing (an absent reference, or an estimate).
enum ReferencedValues {
    None,
    Document(Box<Document>),
    RemovedOwner(Identifier),
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
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [7; 32];
    const POST: [u8; 32] = [1; 32];

    /// Posts nobody changes or deletes, and replies filed under their post's topic
    fn contract() -> DataContract {
        let schemas = platform_value!({
            "post": {
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "topic": { "type": "string", "maxLength": 20, "position": 0 }
                },
                "required": ["topic"],
                "additionalProperties": false
            },
            "reply": {
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": true,
                "indices": [{ "name": "byTopic", "properties": [{ "postId.topic": "asc" }] }],
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "refersTo": { "type": "permanentDocument", "documentType": "post" },
                        "position": 0
                    }
                },
                "required": ["postId"],
                "additionalProperties": false
            }
        });
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned()
    }

    fn document(id: [u8; 32], properties: BTreeMap<String, Value>) -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from(OWNER),
            properties,
            ..Default::default()
        })
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
                BlockInfo::default(),
                apply,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("add document");
    }

    fn replies_on(drive: &Drive, contract: &DataContract, topic: &str) -> Vec<Identifier> {
        let platform_version = PlatformVersion::latest();
        let query = DriveDocumentQuery::from_decomposed_values(
            platform_value!([["postId.topic", "==", topic]]),
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

    /// A reply is keyed by its post's topic on insert and found by it, a dry run of an insert
    /// or a delete prices the post's read without reading it, and a delete finds the entry
    /// under the topic read again.
    #[test]
    fn should_key_a_reply_by_its_posts_topic_and_price_the_read_on_a_dry_run() {
        let platform_version = PlatformVersion::latest();
        let contract = contract();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");
        let post = document(
            POST,
            BTreeMap::from([("topic".to_string(), Value::Text("rust".to_string()))]),
        );
        add(&drive, &contract, "post", &post, true);

        // A dry run reads nothing, so it works before the reply exists
        add(&drive, &contract, "reply", &reply(2), false);
        add(&drive, &contract, "reply", &reply(2), true);
        assert_eq!(
            replies_on(&drive, &contract, "rust"),
            vec![Identifier::from([2; 32])]
        );
        assert!(replies_on(&drive, &contract, "go").is_empty());

        for apply in [false, true] {
            drive
                .delete_document_for_contract(
                    Identifier::from([2; 32]),
                    &contract,
                    "reply",
                    BlockInfo::default(),
                    apply,
                    None,
                    platform_version,
                    None,
                )
                .expect("delete document");
        }
        assert!(replies_on(&drive, &contract, "rust").is_empty());
    }
}
