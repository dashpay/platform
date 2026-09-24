use crate::consensus::basic::data_contract::DocumentTypesAreMissingError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceDeclaration,
    DocumentType,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{DocumentName, TokenConfiguration, TokenContractPosition};
use crate::identity::Purpose;
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentType {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract) fn create_document_types_from_document_schemas_v1(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        has_tokens: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        if document_schemas.is_empty() && !has_tokens {
            return Err(consensus_or_protocol_data_contract_error(
                DocumentTypesAreMissingError::new(data_contract_id).into(),
            ));
        }

        for (name, schema) in document_schemas.into_iter() {
            let document_type = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .structure_version
            {
                0 => DocumentType::try_from_schema(
                    data_contract_id,
                    data_contract_system_version,
                    contract_config_version,
                    &name,
                    schema,
                    schema_defs,
                    token_configurations,
                    data_contact_config,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?,
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "get_document_types_from_value_array_v0 inner document type"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })
                }
            };

            contract_document_types.insert(name.to_string(), document_type);
        }

        // Protocol version 14 and later: an `identityPublicKey` reference's
        // `keyRequirements.boundTo` must name a document type of this contract, and one a
        // key of the required purpose can be bound to. A document type's parse sees only its
        // own schema, so the check runs here, once every document type is parsed, and only
        // under full validation (registration), like the meta-schema: a contract read back
        // from state passed it when it was written. It holds the reference's promise that
        // the write-time check never needs a second contract fetch: the bound the key must
        // carry names the declaring contract and one of its own document types.
        //
        // Inert for every protocol version before 14, which also select this generation:
        // `key_requirements` exists on a parsed reference only where the tables carry
        // `apply_property_reference: Some(_)`, which no version before 14 does (their
        // meta-schemas refuse `refersTo` and their parser ignores it), so the loop below
        // finds no requirement to check there and the output is unchanged.
        if !full_validation {
            return Ok(contract_document_types);
        }

        for (name, document_type) in &contract_document_types {
            for (path, property) in document_type.as_ref().flattened_properties() {
                // Both forms of the key reference carry the same requirements.
                // Only a scalar property can hold either: the typed array
                // parser refuses identityPublicKey on an element
                let key_requirements = match &property.property_type {
                    DocumentPropertyType::IdentifierWithReference(
                        DocumentPropertyReferenceTarget::IdentityPublicKey {
                            key_requirements, ..
                        },
                    ) => key_requirements,
                    DocumentPropertyType::KeyIdWithReference(reference) => {
                        &reference.key_requirements
                    }
                    _ => continue,
                };
                let Some(bound_to) = &key_requirements.bound_to else {
                    continue;
                };
                let Some(bound_document_type) = contract_document_types.get(bound_to) else {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "{name}.{path} refersTo keyRequirements boundTo {bound_to:?} names no document type of this contract"
                        )),
                    ));
                };
                // Only these purposes carry a document type bound, and the two encryption
                // purposes only where the bound type declares that it takes such keys;
                // any other pairing is a requirement no key could ever meet
                let bound_type_takes_the_key = match key_requirements.purpose {
                    None | Some(Purpose::AUTHENTICATION) => true,
                    Some(Purpose::ENCRYPTION) => bound_document_type
                        .as_ref()
                        .requires_identity_encryption_bounded_key()
                        .is_some(),
                    Some(Purpose::DECRYPTION) => bound_document_type
                        .as_ref()
                        .requires_identity_decryption_bounded_key()
                        .is_some(),
                    Some(_) => false,
                };
                if !bound_type_takes_the_key {
                    let purpose = key_requirements
                        .purpose
                        .map(|purpose| purpose.wire_name())
                        .unwrap_or_default();
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "{name}.{path} refersTo keyRequirements requires a {purpose} key bound to document type {bound_to:?}, which no key can be: only authentication, encryption and decryption keys carry a document type bound, and an encryption or decryption key only where the type declares requiresIdentityEncryptionBoundedKey or requiresIdentityDecryptionBoundedKey"
                        )),
                    ));
                }
            }
        }

        // Protocol version 14 and later: a `refersTo` lookup into a document type of this
        // contract must resolve in it: the named index exists and is unique, the keys cover
        // its properties exactly, every source holds the kind of value its index property
        // does, and the key cannot move off the document it found (see
        // `DocumentReferenceLookup::referenced_side_error`). Like the check above it
        // needs every document type of the contract, and runs under full validation only. A
        // lookup into another contract is checked against that contract's state at
        // registration, and a reference naming a document type this contract does not have
        // is left to that validation too, which reports it.
        //
        // The type's `ownerRefersTo` or `creatorRefersTo` declaration, whose lookup key
        // takes the writer or the creator for `"."`, is checked the same way.
        //
        // Inert for every protocol version before 14 for the same reason as the check above:
        // a parsed reference carries a `lookup` only where the tables carry
        // `apply_property_reference: Some(_)`, so the loop below finds none there. The same
        // holds for the leaves of a reference expression (`anyOf` / `allOf`), walked through
        // `leaves_with_paths()`, which parse from the same version only (for a single
        // declaration it is the declaration itself, at an empty path, so the walk and the
        // error are unchanged where no expression exists). Only parser generation 3,
        // selected from protocol version 14, sets an owner or creator reference, so before it
        // `reference_declarations` yields the properties' references alone, in the order the
        // loop walked them, and names them as it did.
        for (name, document_type) in &contract_document_types {
            let declaring = document_type.as_ref();
            // On the writer or the creator, on an identifier property or on the elements
            // of a typed array
            for (holder, reference) in declaring.reference_declarations() {
                let Some(declaration) = reference.target() else {
                    continue;
                };
                // Each leaf of a reference expression is judged as it would be alone,
                // and the error names the leaf (`refersTo anyOf[1] lookup`)
                for (leaf_path, target) in declaration.leaves_with_paths() {
                    let Some(DocumentReferenceDeclaration {
                        contract_id,
                        document_type_name,
                        lookup: Some(lookup),
                        permanent,
                        ..
                    }) = target.as_any_document_reference()
                    else {
                        continue;
                    };
                    if contract_id.is_some_and(|contract_id| contract_id != data_contract_id) {
                        continue;
                    }
                    let Some(referenced_document_type) =
                        contract_document_types.get(document_type_name)
                    else {
                        continue;
                    };
                    // A permanentDocument lookup into a deletable type, or a
                    // deletableDocument lookup into one that forbids deletion, fails that
                    // reference whatever its indexes say: registration reports it
                    // (ReferencedDocumentTypeDeletableError or
                    // ReferencedDocumentTypeNotDeletableError), so the lookup is not judged
                    // against a type it could never reference. A deletableDocument lookup
                    // exists from the same protocol version 14 as every other lookup, so
                    // this stays inert before it
                    let referenced = referenced_document_type.as_ref();
                    let deletable = referenced.documents_can_be_deleted()
                        || referenced.documents_can_be_deleted_by_moderators();
                    if permanent == deletable {
                        continue;
                    }
                    if let Some(reason) = lookup.referenced_side_error(declaring, referenced) {
                        let at = if leaf_path.is_empty() {
                            String::new()
                        } else {
                            format!(" {leaf_path}")
                        };
                        return Err(consensus_or_protocol_data_contract_error(
                            DataContractError::InvalidContractStructure(format!(
                                "document type \"{name}\" {}{at} lookup: {reason}",
                                holder.describe()
                            )),
                        ));
                    }
                }
            }
        }

        // Protocol version 14 and later: a `refersTo: listElement` whose list lives in a
        // document type of this contract must find a list there that holds identifiers and
        // never changes: its documents cannot be deleted, `inList` is a stored typed array of
        // identifiers of it, and the list is fixed once a document is written (see
        // `ListElementReference::referenced_side_error`). The `$id` pair naming the list's
        // document was checked by the document type parse under full validation, and the
        // other agreement pairs are checked at registration as every agreement is; a list in
        // another contract is checked against that contract's state at registration, and one
        // in a document type this contract does not have is left to the reference validation,
        // which reports it. A leaf of a reference expression is judged as it would be alone.
        //
        // Inert for every protocol version before 14 for the same reason as the checks above:
        // a parsed reference is a `listElement` only where the tables carry
        // `apply_property_reference: Some(_)`, so the loop below finds none there.
        for (name, document_type) in &contract_document_types {
            let declaring = document_type.as_ref();
            // On the writer or the creator, on an identifier property or on the elements of
            // a typed array
            for (holder, declaration) in declaring.reference_declarations() {
                let Some(declaration) = declaration.target() else {
                    continue;
                };
                for (leaf_path, target) in declaration.leaves_with_paths() {
                    let Some(reference) = target.as_list_element_reference() else {
                        continue;
                    };
                    if reference
                        .contract_id
                        .is_some_and(|contract_id| contract_id != data_contract_id)
                    {
                        continue;
                    }
                    let Some(referenced_document_type) =
                        contract_document_types.get(&reference.document_type_name)
                    else {
                        continue;
                    };
                    if let Some(reason) =
                        reference.referenced_side_error(referenced_document_type.as_ref())
                    {
                        let at = if leaf_path.is_empty() {
                            String::new()
                        } else {
                            format!(" {leaf_path}")
                        };
                        return Err(consensus_or_protocol_data_contract_error(
                            DataContractError::InvalidContractStructure(format!(
                                "document type \"{name}\" {}{at} listElement: {reason}",
                                holder.describe()
                            )),
                        ));
                    }
                }
            }
        }

        Ok(contract_document_types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::document_type::{
        IdentityKeyReferenceRequirements, KeyIdReference, KeyReferenceIdentityProperty,
    };
    use crate::data_contract::DataContract;
    use crate::identity::Purpose;
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
        PlatformSerializableWithPlatformVersion,
    };
    use platform_value::platform_value;
    use platform_value::string_encoding::Encoding;
    use std::ops::Deref;

    /// A contract with a `joinRequest` type whose `recipientId` references an identity key
    /// with `refers_to`, and a `submittedCharter` type for a bound to name. Both types declare
    /// that they take bound encryption and decryption keys.
    fn contract_value(refers_to: Value) -> Value {
        contract_value_taking_bound_keys(refers_to, true)
    }

    /// [`contract_value`], with or without the two bound key keywords on both types.
    fn contract_value_taking_bound_keys(refers_to: Value, takes_bound_keys: bool) -> Value {
        let mut value = platform_value!({
            "$formatVersion": "1",
            "id": Identifier::from_string("4Bqs6itzfoDXzmgQibYZQABbqYsXmawVf7SKe3mKDQVd", Encoding::Base58).expect("a valid id"),
            "ownerId": Identifier::from_string("2b994p95akyNFKtkDnDvBRUotDbkH54MHwGbhQLr5gcU", Encoding::Base58).expect("a valid id"),
            "version": 1,
            "documentSchemas": {
                "joinRequest": {
                    "type": "object",
                    "properties": {
                        "recipientId": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier",
                            "position": 0,
                            "refersTo": refers_to
                        },
                        "recipientKeyId": {
                            "type": "integer",
                            "minimum": 0,
                            "position": 1
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                },
                "submittedCharter": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "maxLength": 64,
                            "position": 0
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                }
            }
        });
        if takes_bound_keys {
            for document_type_name in ["joinRequest", "submittedCharter"] {
                let path = format!("documentSchemas.{document_type_name}");
                let document_schema = value
                    .get_mut_value_at_path(&path)
                    .expect("the document schema");
                for keyword in [
                    "requiresIdentityEncryptionBoundedKey",
                    "requiresIdentityDecryptionBoundedKey",
                ] {
                    document_schema
                        .insert(keyword.to_string(), Value::U8(2))
                        .expect("the keyword inserts");
                }
            }
        }
        value
    }

    fn recipient_id_target(contract: &DataContract) -> DocumentPropertyType {
        contract
            .document_type_for_name("joinRequest")
            .expect("the joinRequest document type")
            .flattened_properties()
            .get("recipientId")
            .map(|p| p.property_type.clone())
            .expect("the recipientId property")
    }

    #[test]
    fn should_accept_bound_to_naming_a_document_type_of_the_contract() {
        let platform_version = PlatformVersion::latest();

        for bound_to in ["submittedCharter", "joinRequest"] {
            let contract = DataContract::from_value(
                contract_value(platform_value!({
                    "type": "identityPublicKey",
                    "keyIdProperty": "recipientKeyId",
                    "keyRequirements": { "purpose": "decryption", "boundTo": bound_to }
                })),
                true,
                platform_version,
            )
            .expect("the contract should parse");

            assert_eq!(
                recipient_id_target(&contract),
                DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::IdentityPublicKey {
                        key_id_property: "recipientKeyId".to_string(),
                        key_requirements: IdentityKeyReferenceRequirements {
                            purpose: Some(Purpose::DECRYPTION),
                            bound_to: Some(bound_to.to_string()),
                        },
                    }
                )
            );
        }
    }

    /// The invalid contract structure message a contract value is refused with under full
    /// validation; without full validation (a contract read back from state) it parses.
    fn refusal_message(value: Value) -> String {
        let platform_version = PlatformVersion::latest();

        DataContract::from_value(value.clone(), false, platform_version)
            .expect("a contract read back from state is not re-checked");

        let error = DataContract::from_value(value, true, platform_version)
            .expect_err("the contract should be refused");
        let ProtocolError::ConsensusError(consensus_error) = &error else {
            panic!("expected a consensus error, got {error}");
        };
        let ConsensusError::BasicError(BasicError::ContractError(
            DataContractError::InvalidContractStructure(message),
        )) = consensus_error.deref()
        else {
            panic!("expected an invalid contract structure error, got {consensus_error}");
        };
        message.clone()
    }

    #[test]
    fn should_reject_bound_to_naming_a_document_type_the_contract_does_not_have() {
        assert_eq!(
            refusal_message(contract_value(platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "electedCharter" }
            }))),
            "joinRequest.recipientId refersTo keyRequirements boundTo \"electedCharter\" names no document type of this contract"
        );
    }

    /// [`contract_value`] with the reference moved onto the key id property: `recipientKeyId`
    /// carries `refers_to` (an `identityProperty` form) and `recipientId` is a plain identifier.
    fn contract_value_with_key_id_reference(refers_to: Value) -> Value {
        let mut value = contract_value(platform_value!({ "type": "identity" }));
        let recipient_id = value
            .get_mut_value_at_path("documentSchemas.joinRequest.properties.recipientId")
            .expect("the recipientId schema");
        recipient_id
            .remove("refersTo")
            .expect("the identity reference removes");
        let recipient_key_id = value
            .get_mut_value_at_path("documentSchemas.joinRequest.properties.recipientKeyId")
            .expect("the recipientKeyId schema");
        recipient_key_id
            .insert("maximum".to_string(), Value::U64(u64::from(u32::MAX)))
            .expect("the maximum inserts");
        recipient_key_id
            .insert("refersTo".to_string(), refers_to)
            .expect("the reference inserts");
        value
    }

    /// The bound check reads the key id form's requirements too.
    #[test]
    fn should_check_bound_to_on_the_key_id_form() {
        let platform_version = PlatformVersion::latest();

        let contract = DataContract::from_value(
            contract_value_with_key_id_reference(platform_value!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
            })),
            true,
            platform_version,
        )
        .expect("the contract should parse");
        assert_eq!(
            contract
                .document_type_for_name("joinRequest")
                .expect("the joinRequest document type")
                .flattened_properties()
                .get("recipientKeyId")
                .map(|p| p.property_type.clone()),
            Some(DocumentPropertyType::KeyIdWithReference(KeyIdReference {
                identity_property: KeyReferenceIdentityProperty::OwnerId,
                key_requirements: IdentityKeyReferenceRequirements {
                    purpose: Some(Purpose::DECRYPTION),
                    bound_to: Some("submittedCharter".to_string()),
                },
            }))
        );

        assert_eq!(
            refusal_message(contract_value_with_key_id_reference(platform_value!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "electedCharter" }
            }))),
            "joinRequest.recipientKeyId refersTo keyRequirements boundTo \"electedCharter\" names no document type of this contract"
        );
    }

    #[test]
    fn should_reject_a_bound_to_no_key_could_ever_carry() {
        // Only authentication, encryption and decryption keys carry a document type bound
        for purpose in ["transfer", "voting", "owner"] {
            let message = refusal_message(contract_value(platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": purpose, "boundTo": "submittedCharter" }
            })));
            assert!(
                message.starts_with(&format!(
                    "joinRequest.recipientId refersTo keyRequirements requires a {purpose} key bound to document type \"submittedCharter\", which no key can be"
                )),
                "{purpose}: {message}"
            );
        }

        // An encryption or decryption key is bound to a document type only where the type
        // declares that it takes such keys
        for purpose in ["encryption", "decryption"] {
            let message = refusal_message(contract_value_taking_bound_keys(
                platform_value!({
                    "type": "identityPublicKey",
                    "keyIdProperty": "recipientKeyId",
                    "keyRequirements": { "purpose": purpose, "boundTo": "submittedCharter" }
                }),
                false,
            ));
            assert!(
                message.contains("which no key can be"),
                "{purpose}: {message}"
            );
        }

        // Whereas an authentication key, or a key of any purpose, can be bound to a type that
        // declares nothing
        for requirements in [
            platform_value!({ "purpose": "authentication", "boundTo": "submittedCharter" }),
            platform_value!({ "boundTo": "submittedCharter" }),
        ] {
            DataContract::from_value(
                contract_value_taking_bound_keys(
                    platform_value!({
                        "type": "identityPublicKey",
                        "keyIdProperty": "recipientKeyId",
                        "keyRequirements": requirements
                    }),
                    false,
                ),
                true,
                PlatformVersion::latest(),
            )
            .expect("an authentication key can carry any document type bound");
        }
    }

    #[test]
    fn should_refuse_key_requirements_before_protocol_version_14_and_accept_them_at_it() {
        let value = contract_value(platform_value!({
            "type": "identityPublicKey",
            "keyIdProperty": "recipientKeyId",
            "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
        }));

        // The meta-schema of protocol version 13 knows no `refersTo` at all
        let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 exists");
        DataContract::from_value(value.clone(), true, platform_version_13)
            .expect_err("a contract with keyRequirements should be refused before version 14");

        let contract = DataContract::from_value(value, true, PlatformVersion::latest())
            .expect("a contract with keyRequirements should parse at version 14");
        assert!(matches!(
            recipient_id_target(&contract),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey { key_requirements, .. }
            ) if key_requirements.purpose == Some(Purpose::DECRYPTION)
                && key_requirements.bound_to.as_deref() == Some("submittedCharter")
        ));
    }

    /// The requirements ride on the schema, which is what the contract serializes, so a
    /// contract with them and one without both come back as parsed, and the one without
    /// serializes exactly as it did before the keyword existed.
    #[test]
    fn should_round_trip_key_requirements_through_platform_serialization() {
        let platform_version = PlatformVersion::latest();

        for refers_to in [
            platform_value!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId" }),
            platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
            }),
        ] {
            let contract =
                DataContract::from_value(contract_value(refers_to), true, platform_version)
                    .expect("the contract should parse");

            let bytes = contract
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("the contract should serialize");
            let deserialized = DataContract::versioned_deserialize_untrusted(
                bytes.as_slice(),
                false,
                platform_version,
            )
            .expect("the contract should deserialize");

            assert_eq!(
                recipient_id_target(&deserialized),
                recipient_id_target(&contract)
            );
            assert_eq!(
                deserialized
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("the contract should serialize again"),
                bytes
            );
        }
    }
}
