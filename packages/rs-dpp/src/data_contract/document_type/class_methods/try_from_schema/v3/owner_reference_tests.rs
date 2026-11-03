//! `ownerRefersTo` and `creatorRefersTo` (protocol version 14): a document
//! type's own `refersTo` declaration, whose value is the document's
//! `$ownerId`, the writer, or its `$creatorId`, the creator. The parse of the
//! two targets they take and the ones they refuse, the owner and creator
//! rules, the checks of a lookup on both sides, the count against the
//! references a document may carry, the protocol version gate, the platform
//! serialization round trip and the contract update rule.

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

/// [`charter_contract_with`] with `creator_refers_to` declared as
/// `creatorRefersTo` on `resignationRequest`, made transferable or tradeable
/// by `keyword` (a `(keyword, value)` pair), or neither when it is `None`.
fn creator_contract_with(
    creator_refers_to: serde_json::Value,
    keyword: Option<(&str, u8)>,
    version: u32,
) -> serde_json::Value {
    let mut schema = charter_contract_with(serde_json::Value::Null, None, version);
    let resignation_request = &mut schema["documentSchemas"]["resignationRequest"];
    resignation_request["creatorRefersTo"] = creator_refers_to;
    if let Some((keyword, value)) = keyword {
        resignation_request[keyword] = json!(value);
    }
    schema
}

/// [`creator_contract_with`] on a transferable type.
fn creator_contract(creator_refers_to: serde_json::Value) -> serde_json::Value {
    creator_contract_with(creator_refers_to, Some(("transferable", 1)), 1)
}

/// A contract builder declaring the given reference on `resignationRequest`.
type ContractDeclaring = fn(serde_json::Value) -> serde_json::Value;

fn creator_reference(contract: &DataContract) -> Option<DocumentPropertyReferenceTarget> {
    contract
        .document_type_for_name("resignationRequest")
        .expect("the resignationRequest document type")
        .creator_reference()
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
fn should_refuse_an_owner_or_creator_reference_to_a_target_its_identity_can_never_be() {
    let declare: [(&str, ContractDeclaring); 2] = [
        ("ownerRefersTo", charter_contract),
        ("creatorRefersTo", creator_contract),
    ];
    for (keyword, contract_declaring) in declare {
        for (declaration, refusal) in [
            (
                json!({ "type": "contract" }),
                "does not take a contract reference",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "owner": "self" } }),
                "does not take a contract reference",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "note" }),
                "does not take an identityPublicKey reference",
            ),
            (
                json!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
                "does not take an identityPublicKey reference",
            ),
            (
                json!({ "type": "token" }),
                "does not take a token reference",
            ),
            (
                json!({ "type": "permanentDocument", "documentType": "addedModerator" }),
                "takes a document reference only with a lookup",
            ),
            (
                json!({ "type": "deletableDocument", "documentType": "post" }),
                "takes a document reference only with a lookup",
            ),
        ] {
            let schema = contract_declaring(declaration.clone());
            // The parser refuses it on the stored path, where no meta-schema
            // runs
            assert_refused(
                contract_on(schema.clone(), false, PlatformVersion::latest()),
                &format!("{keyword} {refusal}"),
            );
            // and the meta-schema reports it at registration, before the
            // parser reads the keyword, as it reports a malformed `refersTo` on
            // a property
            let error = contract(schema).expect_err("the meta-schema should refuse it");
            assert!(
                is_json_schema_error(&error),
                "{keyword} {declaration}: expected a meta-schema error, got {error}"
            );
        }
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
fn should_count_the_owner_or_creator_reference_against_the_references_a_document_may_carry() {
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

    let over_the_limit = format!(
        "declares references for up to {} values per document",
        u32::from(limit) + 1
    );
    assert_refused(
        contract(charter_contract_with(
            json!({ "type": "identity" }),
            Some(references.clone()),
            1,
        )),
        &over_the_limit,
    );
    let mut creator = creator_contract(json!({ "type": "identity" }));
    creator["documentSchemas"]["resignationRequest"]["properties"]["extra"] = references;
    assert_refused(contract(creator), &over_the_limit);
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

#[test]
fn should_parse_a_creator_reference_on_a_type_that_records_creator_ids() {
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
    for keyword in [("transferable", 1), ("tradeMode", 1)] {
        for (creator_refers_to, expected) in [
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
        ] {
            for full_validation in [true, false] {
                let parsed = contract_on(
                    creator_contract_with(creator_refers_to.clone(), Some(keyword), 1),
                    full_validation,
                    PlatformVersion::latest(),
                )
                .unwrap_or_else(|e| panic!("{creator_refers_to} should parse: {e}"));
                assert_eq!(creator_reference(&parsed).as_ref(), Some(&expected));
                assert_eq!(owner_reference(&parsed), None);
            }
        }
    }
}

/// The creator is only recorded on a type whose documents can change owner;
/// elsewhere it is the owner, which `ownerRefersTo` checks. So a type takes
/// at most one of the two keywords.
#[test]
fn should_refuse_a_creator_reference_on_a_type_that_records_no_creator_ids() {
    let identity = json!({ "type": "identity" });
    let mut both = creator_contract_with(identity.clone(), None, 1);
    both["documentSchemas"]["resignationRequest"]["ownerRefersTo"] = identity.clone();
    let mut both_transferable = creator_contract(identity.clone());
    both_transferable["documentSchemas"]["resignationRequest"]["ownerRefersTo"] = identity.clone();

    for (schema, fragment) in [
        (
            creator_contract_with(identity, None, 1),
            "document type \"resignationRequest\" declares creatorRefersTo, but it records no \
             creator ids",
        ),
        (
            both,
            "document type \"resignationRequest\" declares creatorRefersTo, but it records no \
             creator ids",
        ),
        (
            both_transferable,
            "document type \"resignationRequest\" declares ownerRefersTo, but its documents \
             can be transferred or traded",
        ),
    ] {
        for full_validation in [true, false] {
            assert_refused(
                contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
                fragment,
            );
        }
    }
}

/// The owner of a transferable document moves, so the creator's lookup may not
/// read it, as a property's lookup on such a type may not.
#[test]
fn should_refuse_a_creator_lookup_reading_the_owner() {
    assert_refused(
        contract(creator_contract(permanent_added_moderator(json!({
            "index": "byOwnerMember",
            "keys": { "$ownerId": "$ownerId", "memberId": "." }
        })))),
        "document type \"resignationRequest\" creatorRefersTo lookup: key \"$ownerId\" reads \
         \"$ownerId\"",
    );
}

#[test]
fn should_refuse_creator_refers_to_before_protocol_version_14_and_read_it_at_14() {
    let schema = creator_contract(permanent_added_moderator(added_moderator_lookup()));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the keyword");
    let ignored = contract_on(schema.clone(), false, platform_version_13)
        .expect("protocol version 13 should parse the rest of the contract");
    assert_eq!(creator_reference(&ignored), None);

    let accepted = contract_on(schema, true, PlatformVersion::latest()).expect("parses");
    assert!(matches!(
        creator_reference(&accepted),
        Some(DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. })
    ));
}

#[test]
fn should_round_trip_a_creator_reference_and_refuse_changing_it_on_update() {
    let platform_version = PlatformVersion::latest();
    let lookup = permanent_added_moderator(added_moderator_lookup());

    let original = contract(creator_contract(lookup.clone())).expect("parses");
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract should serialize");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract should deserialize");
    assert_eq!(original, recovered);
    assert_eq!(creator_reference(&original), creator_reference(&recovered));

    let transferable = Some(("transferable", 1));
    let without = charter_contract_with(serde_json::Value::Null, None, 1);
    let mut without_transferable = without.clone();
    without_transferable["documentSchemas"]["resignationRequest"]["transferable"] = json!(1);
    for (before, after, operation) in [
        (
            without_transferable,
            creator_contract_with(lookup.clone(), transferable, 2),
            "add",
        ),
        (
            creator_contract_with(lookup.clone(), transferable, 1),
            creator_contract_with(json!({ "type": "identity" }), transferable, 2),
            "replace",
        ),
    ] {
        let old = contract(before).expect("parses");
        let new = contract(after).expect("parses");
        let result = old
            .validate_update(&new, &BlockInfo::default(), platform_version)
            .expect("the update should be judged");
        assert!(
            result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))
                    if e.operation() == operation
                        && e.property_path().starts_with("/creatorRefersTo")
            )),
            "{operation}: {:?}",
            result.errors
        );
    }
}

/// With `anyOf` / `allOf` the writer or the creator may meet one of several
/// targets: an expression whose every leaf can hold that identity is admitted,
/// and a leaf by id is refused as it is alone, named by where it sits.
#[test]
fn should_parse_an_owner_or_creator_reference_expression_of_identity_capable_leaves() {
    let owner_lookup = permanent_added_moderator(json!({
        "index": "byOwnerMember",
        "keys": { "$ownerId": "$ownerId", "memberId": "." }
    }));
    let expression = json!({
        "anyOf": [permanent_added_moderator(added_moderator_lookup()), owner_lookup]
    });
    for full_validation in [true, false] {
        let parsed = contract_on(
            charter_contract(expression.clone()),
            full_validation,
            PlatformVersion::latest(),
        )
        .expect("an expression of lookups should parse");
        let target = owner_reference(&parsed).expect("the owner reference");
        assert!(matches!(target, DocumentPropertyReferenceTarget::AnyOf(_)));
        assert_eq!(target.leaves().len(), 2);
    }
    let creator_expression = json!({
        "allOf": [
            { "type": "identity" },
            permanent_added_moderator(added_moderator_lookup())
        ]
    });
    let parsed = contract(creator_contract(creator_expression)).expect("parses");
    assert!(matches!(
        creator_reference(&parsed),
        Some(DocumentPropertyReferenceTarget::AllOf(_))
    ));

    let with_leaf_by_id = json!({
        "anyOf": [
            permanent_added_moderator(added_moderator_lookup()),
            { "type": "permanentDocument", "documentType": "addedModerator" }
        ]
    });
    for full_validation in [true, false] {
        assert_refused(
            contract_on(
                charter_contract(with_leaf_by_id.clone()),
                full_validation,
                PlatformVersion::latest(),
            ),
            "ownerRefersTo anyOf[1] takes a document reference only with a lookup",
        );
    }
}

/// The moderation charters' resignation: an added moderator is taken off the team by
/// deleting its addition, so the writer's membership may be a deletable document that exists
/// now. `ownerRefersTo` takes a `deletableDocument` found through a lookup, alone or as an
/// operand; `creatorRefersTo` does not, since the creator's document could be deleted after
/// a transfer, leaving the new owner unable to replace theirs.
#[test]
fn should_parse_an_owner_reference_to_a_deletable_document_found_through_a_lookup() {
    let deletable_added_moderator = json!({
        "type": "deletableDocument",
        "documentType": "addedModerator",
        "lookup": added_moderator_lookup()
    });
    let with_deletable_additions = |mut schema: serde_json::Value| {
        schema["documentSchemas"]["addedModerator"]["canBeDeleted"] = json!(true);
        schema
    };

    for owner_refers_to in [
        deletable_added_moderator.clone(),
        json!({ "anyOf": [{ "type": "identity" }, deletable_added_moderator.clone()] }),
    ] {
        for full_validation in [true, false] {
            let parsed = contract_on(
                with_deletable_additions(charter_contract(owner_refers_to.clone())),
                full_validation,
                PlatformVersion::latest(),
            )
            .unwrap_or_else(|e| panic!("{owner_refers_to} should parse: {e}"));
            let target = owner_reference(&parsed).expect("the owner reference");
            assert!(
                target.leaves().into_iter().any(|leaf| matches!(
                    leaf,
                    DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                        document_type_name,
                        ..
                    } if document_type_name == "addedModerator"
                )),
                "{owner_refers_to}: {target:?}"
            );
        }
    }

    // The meta-schema refuses it on the creator at registration, and the parser on the
    // stored path, alone or as an operand
    let schema = with_deletable_additions(creator_contract(deletable_added_moderator.clone()));
    let error = contract(schema.clone()).expect_err("the meta-schema should refuse it");
    assert!(
        is_json_schema_error(&error),
        "expected a meta-schema error, got {error}"
    );
    assert_refused(
        contract_on(schema, false, PlatformVersion::latest()),
        "creatorRefersTo does not take a deletableDocument reference",
    );
    assert_refused(
        contract_on(
            with_deletable_additions(creator_contract(json!({
                "anyOf": [{ "type": "identity" }, deletable_added_moderator]
            }))),
            false,
            PlatformVersion::latest(),
        ),
        "creatorRefersTo anyOf[1] does not take a deletableDocument reference",
    );
}
