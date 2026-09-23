//! `ownerRefersTo` (protocol version 14): a document type's own `refersTo`
//! declaration, whose value is the document's `$ownerId`, the writer. The
//! parse of the two targets it takes and the ones it refuses, the owner rule,
//! the checks of its lookup on both sides, its count against the references a
//! document may carry, the protocol version gate, the platform serialization
//! round trip and the contract update rule.

use crate::block::block_info::BlockInfo;
use crate::consensus::basic::basic_error::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentReferenceLookup, LookupKeySource,
};
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [7; 32];

fn identifier(position: u32) -> serde_json::Value {
    json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

/// The moderation charters' rule: the writer must be the `memberId` of an
/// `addedModerator` for the document's `electedCharterId`.
fn added_moderator_lookup() -> serde_json::Value {
    json!({
        "index": "byElectedCharterMember",
        "keys": { "electedCharterId": "electedCharterId", "memberId": "." }
    })
}

fn permanent_added_moderator(lookup: serde_json::Value) -> serde_json::Value {
    json!({ "type": "permanentDocument", "documentType": "addedModerator", "lookup": lookup })
}

/// A contract with a permanent, immutable `addedModerator` type (unique on
/// (`electedCharterId`, `memberId`) and on (`$ownerId`, `memberId`), with a
/// non-unique `byMember` index), a deletable `post` type, and a
/// `resignationRequest` type declaring `owner_refers_to` (when it is not
/// null) next to a required `electedCharterId`, an optional `note` and
/// `extra`, one more property schema.
fn charter_contract_with(
    owner_refers_to: serde_json::Value,
    extra: Option<serde_json::Value>,
    version: u32,
) -> serde_json::Value {
    let mut resignation_request = json!({
        "type": "object",
        "properties": {
            "electedCharterId": identifier(0),
            "note": { "type": "string", "maxLength": 63, "position": 1 }
        },
        "required": ["electedCharterId"],
        "additionalProperties": false
    });
    if !owner_refers_to.is_null() {
        resignation_request["ownerRefersTo"] = owner_refers_to;
    }
    if let Some(extra) = extra {
        resignation_request["properties"]["extra"] = extra;
    }
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": version,
        "documentSchemas": {
            "addedModerator": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "electedCharterId": identifier(0),
                    "memberId": identifier(1)
                },
                "indices": [
                    {
                        "name": "byElectedCharterMember",
                        "properties": [{ "electedCharterId": "asc" }, { "memberId": "asc" }],
                        "unique": true
                    },
                    {
                        "name": "byOwnerMember",
                        "properties": [{ "$ownerId": "asc" }, { "memberId": "asc" }],
                        "unique": true
                    },
                    { "name": "byMember", "properties": [{ "memberId": "asc" }] }
                ],
                "required": ["electedCharterId", "memberId"],
                "additionalProperties": false
            },
            "post": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            },
            "resignationRequest": resignation_request
        }
    })
}

fn charter_contract(owner_refers_to: serde_json::Value) -> serde_json::Value {
    charter_contract_with(owner_refers_to, None, 1)
}

fn contract_on(
    contract: serde_json::Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = platform_value::to_value(contract).expect("the contract should convert");
    DataContract::from_value(value, full_validation, platform_version)
}

fn contract(contract: serde_json::Value) -> Result<DataContract, ProtocolError> {
    contract_on(contract, true, PlatformVersion::latest())
}

fn owner_reference(contract: &DataContract) -> Option<DocumentPropertyReferenceTarget> {
    contract
        .document_type_for_name("resignationRequest")
        .expect("the resignationRequest document type")
        .owner_reference()
        .cloned()
}

fn is_json_schema_error(error: &ProtocolError) -> bool {
    matches!(
        error,
        ProtocolError::ConsensusError(boxed)
            if matches!(**boxed, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

fn assert_refused(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract should be refused");
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
}

#[test]
fn should_parse_an_identity_or_a_permanent_document_lookup_owner_reference() {
    let lookup = DocumentReferenceLookup {
        index: "byElectedCharterMember".to_string(),
        keys: BTreeMap::from([
            (
                "electedCharterId".to_string(),
                LookupKeySource::Property("electedCharterId".to_string()),
            ),
            ("memberId".to_string(), LookupKeySource::ReferenceValue),
        ]),
    };
    let agreement = BTreeMap::from([("$ownerId".to_string(), "memberId".to_string())]);

    for (owner_refers_to, expected) in [
        (
            json!({ "type": "identity" }),
            DocumentPropertyReferenceTarget::Identity,
        ),
        (
            permanent_added_moderator(added_moderator_lookup()),
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "addedModerator".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: lookup.clone(),
            },
        ),
        // The referring side of an agreement may be the writer: the same
        // identity as the reference's value
        (
            json!({
                "type": "permanentDocument",
                "documentType": "addedModerator",
                "propertyAgreement": { "$ownerId": "memberId" },
                "lookup": added_moderator_lookup()
            }),
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "addedModerator".to_string(),
                property_agreement: agreement,
                lookup,
            },
        ),
    ] {
        // Read on both paths, as every doctype-level keyword of generation 3
        for full_validation in [true, false] {
            let parsed = contract_on(
                charter_contract(owner_refers_to.clone()),
                full_validation,
                PlatformVersion::latest(),
            )
            .unwrap_or_else(|e| panic!("{owner_refers_to} should parse: {e}"));
            assert_eq!(
                owner_reference(&parsed).as_ref(),
                Some(&expected),
                "{owner_refers_to}"
            );
        }
    }

    // A type that declares none has none, and no other type gains one
    let without = contract(charter_contract(serde_json::Value::Null)).expect("parses");
    assert_eq!(owner_reference(&without), None);
    assert_eq!(
        without
            .document_type_for_name("addedModerator")
            .expect("the addedModerator document type")
            .owner_reference(),
        None
    );
}

/// The targets the writer's identity id can never be, and `identityPublicKey`,
/// which needs a key id the writer does not carry: a type declaring one could
/// never be written.
#[test]
fn should_refuse_an_owner_reference_to_a_target_the_writer_can_never_be() {
    for (owner_refers_to, fragment) in [
        (
            json!({ "type": "contract" }),
            "ownerRefersTo does not take a contract reference",
        ),
        (
            json!({ "type": "contract", "contractRequirements": { "owner": "self" } }),
            "ownerRefersTo does not take a contract reference",
        ),
        (
            json!({ "type": "identityPublicKey", "keyIdProperty": "note" }),
            "ownerRefersTo does not take an identityPublicKey reference",
        ),
        (
            json!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
            "ownerRefersTo does not take an identityPublicKey reference",
        ),
        (
            json!({ "type": "token" }),
            "ownerRefersTo does not take a token reference",
        ),
        (
            json!({ "type": "permanentDocument", "documentType": "addedModerator" }),
            "ownerRefersTo takes a permanentDocument reference only with a lookup",
        ),
        (
            json!({ "type": "deletableDocument", "documentType": "post" }),
            "ownerRefersTo does not take a deletableDocument reference",
        ),
    ] {
        let schema = charter_contract(owner_refers_to.clone());
        // The parser refuses it on the stored path, where no meta-schema runs
        assert_refused(
            contract_on(schema.clone(), false, PlatformVersion::latest()),
            fragment,
        );
        // and the meta-schema reports it at registration, before the parser
        // reads the keyword, as it reports a malformed `refersTo` on a property
        let error = contract(schema).expect_err("the meta-schema should refuse it");
        assert!(
            is_json_schema_error(&error),
            "{owner_refers_to}: expected a meta-schema error, got {error}"
        );
    }
}

/// A transfer or a purchase would hand a document to an owner the declaration
/// never checked, so the type must keep its writer as its owner.
#[test]
fn should_refuse_an_owner_reference_on_a_type_whose_documents_can_change_owner() {
    for (keyword, value) in [("transferable", 1), ("tradeMode", 1)] {
        let mut schema = charter_contract(json!({ "type": "identity" }));
        schema["documentSchemas"]["resignationRequest"][keyword] = json!(value);
        for full_validation in [true, false] {
            assert_refused(
                contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
                "document type \"resignationRequest\" declares ownerRefersTo, but its documents \
                 can be transferred or traded",
            );
        }
    }
}

#[test]
fn should_refuse_an_owner_lookup_without_the_reference_value() {
    // `"$ownerId"` names the writer too, but a lookup still fills exactly one
    // key part from `"."`
    for keys in [
        json!({ "electedCharterId": "electedCharterId", "memberId": "$ownerId" }),
        json!({ "electedCharterId": ".", "memberId": "." }),
    ] {
        let schema = charter_contract(permanent_added_moderator(json!({
            "index": "byElectedCharterMember",
            "keys": keys
        })));
        for full_validation in [true, false] {
            assert_refused(
                contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
                "must fill exactly one index property from \".\"",
            );
        }
    }
}

#[test]
fn should_check_the_referring_side_of_an_owner_lookup_on_every_parse() {
    // A key part read from an optional property could be missing
    let optional_source = charter_contract(permanent_added_moderator(json!({
        "index": "byElectedCharterMember",
        "keys": { "electedCharterId": "note", "memberId": "." }
    })));
    for full_validation in [true, false] {
        assert_refused(
            contract_on(
                optional_source.clone(),
                full_validation,
                PlatformVersion::latest(),
            ),
            "document type \"resignationRequest\" ownerRefersTo lookup: key \
             \"electedCharterId\" reads \"note\", which is not required",
        );
    }

    // `"$ownerId"` is the writer, like `"."`, and passes the owner rule of a
    // lookup's referring side: the type cannot change owner
    let writer_source = contract(charter_contract(permanent_added_moderator(json!({
        "index": "byOwnerMember",
        "keys": { "$ownerId": "$ownerId", "memberId": "." }
    }))))
    .expect("an owner lookup may read the writer");
    assert!(matches!(
        owner_reference(&writer_source),
        Some(DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. })
    ));
}

#[test]
fn should_check_an_owner_lookup_into_a_type_of_the_same_contract() {
    for (index, fragment) in [
        (
            "byNothing",
            "document type \"resignationRequest\" ownerRefersTo lookup: the referenced document \
             type \"addedModerator\" has no index named \"byNothing\"",
        ),
        (
            "byMember",
            "document type \"resignationRequest\" ownerRefersTo lookup: index \"byMember\" of \
             \"addedModerator\" is not unique",
        ),
    ] {
        let keys = if index == "byMember" {
            json!({ "memberId": "." })
        } else {
            json!({ "anything": "." })
        };
        let schema = charter_contract(permanent_added_moderator(
            json!({ "index": index, "keys": keys }),
        ));
        assert_refused(contract(schema.clone()), fragment);
        // A contract read back from state passed the check when it was
        // registered
        contract_on(schema, false, PlatformVersion::latest())
            .expect("the stored path does not re-check the referenced side");
    }
}

#[test]
fn should_count_the_owner_reference_against_the_references_a_document_may_carry() {
    let limit = PlatformVersion::latest()
        .system_limits
        .max_references_per_document;
    // A typed array of as many identity references as a document may carry
    let references = json!({
        "type": "array",
        "minItems": 0,
        "maxItems": limit,
        "items": {
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "refersTo": { "type": "identity" }
        },
        "position": 2
    });

    contract(charter_contract_with(
        serde_json::Value::Null,
        Some(references.clone()),
        1,
    ))
    .expect("the array alone is at the limit");

    assert_refused(
        contract(charter_contract_with(
            json!({ "type": "identity" }),
            Some(references),
            1,
        )),
        &format!(
            "declares references for up to {} values per document",
            u32::from(limit) + 1
        ),
    );
}

#[test]
fn should_refuse_owner_refers_to_before_protocol_version_14_and_read_it_at_14() {
    let schema = charter_contract(permanent_added_moderator(added_moderator_lookup()));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 closes the document type level, so a registering parse
    // refuses the unknown keyword
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the keyword");
    // A parser predating it does not read it
    let ignored = contract_on(schema.clone(), false, platform_version_13)
        .expect("protocol version 13 should parse the rest of the contract");
    assert_eq!(owner_reference(&ignored), None);

    let accepted = contract_on(schema, true, PlatformVersion::latest()).expect("parses");
    assert!(matches!(
        owner_reference(&accepted),
        Some(DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. })
    ));
}

/// The declaration rides on the schema, which is what the contract
/// serializes: a contract with one comes back with it, and one without
/// serializes as it did before the keyword existed, its schema carrying no
/// trace of it.
#[test]
fn should_round_trip_a_contract_through_platform_serialization_with_and_without_an_owner_reference()
{
    let platform_version = PlatformVersion::latest();

    for owner_refers_to in [
        serde_json::Value::Null,
        json!({ "type": "identity" }),
        permanent_added_moderator(added_moderator_lookup()),
    ] {
        let original = contract(charter_contract(owner_refers_to.clone())).expect("parses");
        let bytes = original
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the contract should serialize");
        let recovered =
            DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
                .expect("the contract should deserialize");

        assert_eq!(original, recovered, "ownerRefersTo {owner_refers_to}");
        assert_eq!(owner_reference(&original), owner_reference(&recovered));
        assert_eq!(
            recovered
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("the contract should serialize again"),
            bytes
        );
        let stored_schema = recovered
            .document_schemas()
            .get("resignationRequest")
            .copied()
            .expect("the resignationRequest schema");
        assert_eq!(
            stored_schema
                .get_optional_value("ownerRefersTo")
                .expect("the schema is a map")
                .is_some(),
            !owner_refers_to.is_null()
        );
    }
}

#[test]
fn should_refuse_adding_removing_or_changing_an_owner_reference_on_update() {
    let platform_version = PlatformVersion::latest();
    let identity = json!({ "type": "identity" });
    let lookup = permanent_added_moderator(added_moderator_lookup());

    for (before, after, operation) in [
        (serde_json::Value::Null, identity.clone(), "add"),
        (lookup.clone(), serde_json::Value::Null, "remove"),
        (identity.clone(), lookup.clone(), "replace"),
    ] {
        let old = contract(charter_contract_with(before.clone(), None, 1)).expect("parses");
        let new = contract(charter_contract_with(after.clone(), None, 2)).expect("parses");
        let result = old
            .validate_update(&new, &BlockInfo::default(), platform_version)
            .expect("the update should be judged");
        let incompatible: Vec<_> = result
            .errors
            .iter()
            .map(|error| match error {
                ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))
                    if e.document_type_name() == "resignationRequest"
                        && e.property_path().starts_with("/ownerRefersTo") =>
                {
                    e.operation()
                }
                other => panic!("{before} -> {after}: unexpected {other:?}"),
            })
            .collect();
        assert!(
            incompatible.contains(&operation),
            "{before} -> {after}: {:?}",
            result.errors
        );
    }

    // An update leaving it as it was is judged on the rest alone
    let old = contract(charter_contract_with(lookup.clone(), None, 1)).expect("parses");
    let new = contract(charter_contract_with(lookup, None, 2)).expect("parses");
    let result = old
        .validate_update(&new, &BlockInfo::default(), platform_version)
        .expect("the update should be judged");
    assert!(result.is_valid(), "{:?}", result.errors);
}
