//! The native schema checks against the document meta-schema they replace, and registration
//! with them (`check_document_type_schema` 1) against registration with the meta-schema
//! check (0), on hand-written document types, on every document type of the contract corpus
//! and on schemas generated from both.

use super::super::common::select_document_meta_schema;
use super::document_type::DOCUMENT_SCHEMA_URI;
use super::properties::IDENTIFIER_MEDIA_TYPE;
use super::Checker;
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::schema::validation_schema::ValidationSchema;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::{DataContract, TokenConfiguration, TokenContractPosition};
use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
use crate::ProtocolError;
use platform_value::string_encoding::{decode, Encoding};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;
use std::sync::OnceLock;

// ---- what the comparisons run ----

/// The first error of the document meta-schema check on `root`, as its instance path.
fn meta_schema_refusal(root: &JsonValue) -> Option<String> {
    let meta_schema = select_document_meta_schema(3, "native schema tests")
        .expect("expected the document meta-schema");
    let refusal = meta_schema
        .validate(root)
        .err()
        .and_then(|mut errors| errors.next())
        .map(|error| error.instance_path.to_string());
    refusal
}

/// The native check of `root`.
fn native_refusal(root: &JsonValue) -> Option<JsonSchemaError> {
    match Checker::new(PlatformVersion::latest()).root(root) {
        Ok(()) => None,
        Err(ProtocolError::ConsensusError(error)) => match *error {
            ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => Some(error),
            other => panic!("expected a JSON Schema error, got {other:?}"),
        },
        Err(other) => panic!("expected a consensus error, got {other:?}"),
    }
}

/// Whether a refusal is the one rule the native checks hold that the meta-schema did not:
/// an index property name longer than GroveDB's 255-byte keys.
fn is_index_property_name_cap(error: &JsonSchemaError) -> bool {
    error.keyword() == "propertyNames" && error.error_summary().contains("bytes")
}

/// A document type and what its contract gives its parse.
#[derive(Clone)]
struct Case {
    origin: String,
    name: String,
    schema: JsonValue,
    defs: Option<JsonValue>,
    contract_id: Identifier,
    config: DataContractConfig,
    tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,
}

impl Case {
    fn schema_defs(&self) -> Option<Option<BTreeMap<String, Value>>> {
        match &self.defs {
            None => Some(None),
            Some(JsonValue::Object(defs)) => Some(Some(
                defs.iter()
                    .map(|(name, schema)| (name.clone(), Value::from(schema.clone())))
                    .collect(),
            )),
            Some(_) => None,
        }
    }

    /// The schema the parse checks: expanded, with the base schema and the contract's
    /// expanded `$defs` added. `None` where the parse refuses before it checks the schema.
    fn root(&self, platform_version: &PlatformVersion) -> Option<Value> {
        let schema_defs = self.schema_defs()?;
        let schema = Value::from(self.schema.clone());
        let expanded_defs = match &schema_defs {
            Some(defs) => DocumentType::expand_schema_defs_property_type_shorthands(
                defs,
                true,
                platform_version,
            )
            .ok()?
            .or(Some(defs.clone())),
            None => None,
        };
        let schema = DocumentType::expand_property_type_shorthands(&schema, true, platform_version)
            .ok()?
            .unwrap_or(schema);
        DocumentType::enrich_with_base_schema(
            schema,
            expanded_defs.map(Value::from),
            platform_version,
        )
        .ok()
    }

    /// The schema the meta-schema check read.
    fn validating_root(&self, platform_version: &PlatformVersion) -> Option<JsonValue> {
        self.root(platform_version)?.try_to_validating_json().ok()
    }

    /// The document type registered at the latest protocol version, with its schema checked
    /// natively, or against the meta-schema.
    fn parse(&self, native: bool) -> Option<Result<DocumentType, ProtocolError>> {
        let platform_version = if native {
            PlatformVersion::latest()
        } else {
            latest_with_the_meta_schema_check()
        };
        let schema_defs = self.schema_defs()?;
        let schema = Value::from(self.schema.clone());
        Some(DocumentType::try_from_schema(
            self.contract_id,
            1,
            self.config.version(),
            &self.name,
            schema,
            schema_defs.as_ref(),
            &self.tokens,
            &self.config,
            true,
            &mut vec![],
            platform_version,
        ))
    }

    /// Whether the typed validation schema builder refuses this document type.
    fn refused_by_the_builder(&self) -> bool {
        self.root(PlatformVersion::latest())
            .is_some_and(|root| ValidationSchema::try_from_root(&root).is_err())
    }
}

fn consensus_error(error: &ProtocolError) -> Option<&ConsensusError> {
    match error {
        ProtocolError::ConsensusError(error) => Some(error),
        _ => None,
    }
}

fn json_schema_error(error: &ProtocolError) -> Option<&JsonSchemaError> {
    match consensus_error(error)? {
        ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => Some(error),
        _ => None,
    }
}

/// The latest protocol version with `check_document_type_schema` 0: registration checks a
/// schema against the meta-schema through the `jsonschema` crate.
fn latest_with_the_meta_schema_check() -> &'static PlatformVersion {
    static VERSION: OnceLock<PlatformVersion> = OnceLock::new();
    VERSION.get_or_init(|| {
        let mut version = PlatformVersion::latest().clone();
        version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .check_document_type_schema = 0;
        version
    })
}

/// How the native checks compared with the meta-schema, and registration with them with
/// registration with the meta-schema check, over a set of cases.
#[derive(Default, Debug)]
struct Tally {
    checked: usize,
    both_accept: usize,
    both_refuse: usize,
    capped: usize,
    parsed: usize,
    parse_refusals: usize,
    builder_refusals: usize,
    error_class_moves: usize,
}

/// Compares the native checks with the meta-schema on `case`, and registering it with them
/// with registering it with the meta-schema check. Returns what differs that may not.
fn compare(case: &Case, tally: &mut Tally) -> Vec<String> {
    let platform_version = PlatformVersion::latest();
    let mut differences = Vec::new();
    let origin = &case.origin;

    if let Some(root) = case.validating_root(platform_version) {
        tally.checked += 1;
        match (meta_schema_refusal(&root), native_refusal(&root)) {
            (None, None) => tally.both_accept += 1,
            (Some(_), Some(_)) => tally.both_refuse += 1,
            (None, Some(native)) if is_index_property_name_cap(&native) => tally.capped += 1,
            (None, Some(native)) => differences.push(format!(
                "{origin}: the native checks refuse what the meta-schema accepts: {native}\n{root:#}"
            )),
            (Some(path), None) => differences.push(format!(
                "{origin}: the native checks accept what the meta-schema refuses at {path}\n{root:#}"
            )),
        }
    }

    let (Some(with_meta), Some(with_native)) = (case.parse(false), case.parse(true)) else {
        return differences;
    };
    tally.parsed += 1;
    match (with_meta, with_native) {
        (Ok(meta), Ok(native)) => {
            if meta != native {
                differences.push(format!("{origin}: the checks parse different types"));
            }
        }
        (Err(meta), Ok(_)) => differences.push(format!(
            "{origin}: the native check admits what the meta-schema check refuses: {meta}\n{:#}",
            case.schema
        )),
        (Ok(_), Err(native)) => {
            if json_schema_error(&native).is_some_and(is_index_property_name_cap) {
                tally.capped += 1;
            } else if case.refused_by_the_builder() {
                tally.builder_refusals += 1;
            } else {
                differences.push(format!(
                    "{origin}: the native check refuses what the meta-schema check admits: \
                     {native}\n{:#}",
                    case.schema
                ));
            }
        }
        (Err(meta), Err(native)) => {
            tally.parse_refusals += 1;
            let code = |error: &ProtocolError| consensus_error(error).map(ErrorWithCode::code);
            if code(&meta) != code(&native) {
                tally.error_class_moves += 1;
                // The refusals that may change class: a refusal of the `jsonschema` compile,
                // which the builder now makes, and the byte cap
                let moved = (json_schema_error(&meta).is_some()
                    && case
                        .validating_root(platform_version)
                        .is_some_and(|root| meta_schema_refusal(&root).is_none()))
                    || json_schema_error(&native).is_some_and(is_index_property_name_cap);
                if !moved {
                    differences.push(format!(
                        "{origin}: the native check refuses with code {:?} ({native}) where the \
                         meta-schema check refused with {:?} ({meta})\n{:#}",
                        code(&native),
                        code(&meta),
                        case.schema
                    ));
                }
            }
        }
    }
    differences
}

// ---- the cases ----

fn moderated_config(platform_version: &PlatformVersion) -> DataContractConfig {
    DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config")
        .with_moderation(Some(ContractModerationConfig {
            banlist: true,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }))
}

fn identifier_property(position: u64) -> JsonValue {
    json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": IDENTIFIER_MEDIA_TYPE,
        "position": position,
    })
}

/// Hand-written document types using the keywords the corpus uses little or not at all.
fn written_cases() -> Vec<Case> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    let mut user = identifier_property(0);
    user["refersTo"] = json!({ "type": "identity" });
    let mut post = identifier_property(1);
    post["refersTo"] = json!({
        "type": "permanentDocument",
        "contractId": "6hVQW16jyvZyGSQk2YVty4ND6bgFXozizYWnPt753uW5",
        "documentType": "post",
    });
    let schemas = [
        (
            "profile",
            json!({
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "documentsKeepHistory": false,
                "signatureSecurityLevelRequirement": 2,
                "transferable": 0,
                "tradeMode": 0,
                "creationRestrictionMode": 0,
                "requiresIdentityEncryptionBoundedKey": 0,
                "requiresIdentityDecryptionBoundedKey": 0,
                "indices": [
                    {
                        "name": "byOwnerName",
                        "properties": [{ "$ownerId": "asc" }, { "name": "asc" }],
                        "unique": true,
                    },
                    {
                        "name": "byScore",
                        "properties": [{ "score": "asc" }],
                    },
                ],
                "properties": {
                    "name": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 63,
                        "pattern": "^[a-z]+$",
                        "position": 0,
                    },
                    "score": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 },
                    "avatar": identifier_property(2),
                    "tags": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 20 },
                        "maxItems": 5,
                        "position": 3,
                    },
                    "meta": {
                        "type": "object",
                        "properties": {
                            "a": { "type": "string", "maxLength": 5, "position": 0 },
                        },
                        "additionalProperties": false,
                        "position": 4,
                    },
                    "kind": { "type": "string", "enum": ["a", "b"], "maxLength": 1, "position": 5 },
                    "label": { "$ref": "#/$defs/label", "position": 6 },
                    "note": {
                        "type": "string",
                        "maxLength": 100,
                        "description": "a note",
                        "$comment": "free text",
                        "position": 7,
                    },
                },
                "required": ["name", "$createdAt"],
                "additionalProperties": false,
            }),
        ),
        (
            "reference",
            json!({
                "type": "object",
                "properties": {
                    "user": user,
                    "post": post,
                    "friends": {
                        "type": "array",
                        "items": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": IDENTIFIER_MEDIA_TYPE,
                            "refersTo": { "type": "identity" },
                        },
                        "maxItems": 3,
                        "position": 2,
                    },
                    "secret": {
                        "type": "array",
                        "byteArray": true,
                        "maxItems": 64,
                        "encryptedFor": {
                            "recipient": "user",
                            "recipientKey": "recipientKeyId",
                            "senderKey": "senderKeyId",
                            "scheme": "ecdh-secp256k1-aes256-cbc",
                        },
                        "position": 3,
                    },
                    "recipientKeyId": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4294967295_u64,
                        "position": 4,
                    },
                    "senderKeyId": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4294967295_u64,
                        "position": 5,
                    },
                },
                "required": ["user"],
                "additionalProperties": false,
            }),
        ),
        (
            "summed",
            json!({
                "type": "object",
                "documentsCountable": true,
                "documentsSummable": "amount",
                "rangeSummable": true,
                "indices": [
                    {
                        "name": "byDay",
                        "properties": [{ "day": "asc" }],
                        "countable": "countableAllowingOffset",
                        "rangeCountable": true,
                        "summable": "amount",
                        "rangeSummable": true,
                    },
                ],
                "properties": {
                    "day": { "type": "integer", "minimum": 0, "maximum": 100000, "position": 0 },
                    "amount": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
                    "slug": {
                        "type": "string",
                        "maxLength": 63,
                        "generatedFrom": {
                            "function": "sys.stringTransformations.lowercase",
                            "params": ["title"],
                        },
                        "position": 2,
                    },
                    "title": { "type": "string", "maxLength": 63, "maxBytes": 100, "position": 3 },
                },
                "required": ["day", "amount"],
                "additionalProperties": false,
            }),
        ),
        (
            "constrained",
            json!({
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": false,
                "ttl": 3600,
                "transient": ["draft"],
                "immutable": ["text", { "property": "price", "when": { "present": "locked" } }],
                "propertyConstraints": { "cheap": { "lessThanOrEqual": ["price", 100] } },
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "deleted": { "type": "boolean", "position": 1 },
                    "price": { "type": "integer", "minimum": 0, "position": 2 },
                    "locked": { "type": "boolean", "position": 3 },
                    "draft": { "type": "string", "maxLength": 10, "position": 4 },
                    "late": { "type": "string", "maxLength": 10, "requiredSince": 2, "position": 5 },
                },
                "required": ["late", "$createdAt"],
                "additionalProperties": false,
            }),
        ),
        (
            "fees",
            json!({
                "type": "object",
                "actionFees": {
                    "pricing": "fixed",
                    "create": { "owner": 1000 },
                    "delete": { "owner": 0, "moderators": 10 },
                },
                "tokenCost": {
                    "create": {
                        "contractId": vec![7; 32],
                        "tokenPosition": 0,
                        "amount": 10,
                        "effect": 0,
                        "gasFeesPaidBy": 0,
                    },
                },
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
            }),
        ),
        (
            "windowed",
            json!({
                "type": "object",
                "documentsCountable": true,
                "indices": [
                    {
                        "name": "byHour",
                        "properties": [{ "$createdAt": "asc" }],
                        "countable": "countable",
                        "timeRange": { "on": "$createdAt", "range": 3600000, "step": 3600000 },
                    },
                    {
                        "name": "byLevel",
                        "properties": [{ "level": "asc" }],
                        "countable": true,
                        "rangeCountable": true,
                        "rankedCountable": true,
                    },
                ],
                "properties": {
                    "level": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
                },
                "required": ["$createdAt"],
                "additionalProperties": false,
            }),
        ),
        (
            "likes",
            json!({
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": IDENTIFIER_MEDIA_TYPE,
                        "refersTo": { "type": "identity" },
                        "position": 1,
                    },
                },
                "required": ["hashtag", "postId"],
                "indices": [
                    {
                        "name": "byHashtagPost",
                        "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
                        "terminal": "$ownerId",
                        "countable": true,
                        "rangeCountable": true,
                        "rankedCountable": true,
                    },
                    {
                        "name": "byLiker",
                        "properties": [{ "$ownerId": "asc" }],
                        "terminal": "postId",
                    },
                ],
                "additionalProperties": false,
            }),
        ),
        (
            "response",
            json!({
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "canBeDeleted": true,
                "indices": [{ "name": "byRequest", "terminal": ["requestHash", "$ownerId"] }],
                "entryPayload": ["payload"],
                "properties": {
                    "requestHash": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 20,
                        "maxItems": 20,
                        "position": 0,
                    },
                    "payload": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 1,
                        "maxItems": 100,
                        "position": 1,
                    },
                },
                "required": ["requestHash", "payload"],
                "additionalProperties": false,
            }),
        ),
    ];
    let mut cases: Vec<Case> = schemas
        .into_iter()
        .map(|(name, schema)| Case {
            origin: format!("written {name}"),
            name: name.to_string(),
            schema,
            defs: Some(json!({ "label": { "type": "string", "maxLength": 10 } })),
            contract_id: Identifier::new([1; 32]),
            config: config.clone(),
            tokens: BTreeMap::new(),
        })
        .collect();
    cases.push(Case {
        origin: "written moderated".to_string(),
        name: "moderated".to_string(),
        schema: json!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "moderatorAbilities": {
                "delete": true,
                "deleteWithin": 86400,
                "deleteKeepsRecord": true,
                "deleteKeepsFields": ["text"],
            },
            "retractedWhen": { "present": "deleted" },
            "properties": {
                "text": { "type": "string", "maxLength": 50, "position": 0 },
                "deleted": { "type": "boolean", "position": 1 },
            },
            "required": ["$updatedAt"],
            "additionalProperties": false,
        }),
        defs: None,
        contract_id: Identifier::new([1; 32]),
        config: moderated_config(platform_version),
        tokens: BTreeMap::new(),
    });
    cases
}

/// Every document type of the contract corpus (`tests/fixtures/contract_corpus`).
fn corpus_cases() -> Vec<Case> {
    let platform_version = PlatformVersion::latest();
    let lines = include_str!("../../../../../../tests/fixtures/contract_corpus/mainnet.jsonl")
        .lines()
        .chain(
            include_str!("../../../../../../tests/fixtures/contract_corpus/testnet.jsonl").lines(),
        );
    let mut cases = Vec::new();
    for line in lines {
        let line: JsonValue = serde_json::from_str(line).expect("expected a JSON corpus line");
        let id = line["id"].as_str().expect("expected a contract id");
        let bytes = decode(
            line["data"].as_str().expect("expected contract bytes"),
            Encoding::Base64,
        )
        .expect("expected base64 contract bytes");
        let contract = DataContract::versioned_deserialize_trusted(&bytes, false, platform_version)
            .expect("expected a corpus contract to load");
        let defs = contract.schema_defs().map(|defs| {
            JsonValue::Object(
                defs.iter()
                    .map(|(name, schema)| {
                        let schema = schema
                            .try_to_validating_json()
                            .expect("expected JSON definitions");
                        (name.clone(), schema)
                    })
                    .collect(),
            )
        });
        for (name, document_type) in contract.document_types() {
            cases.push(Case {
                origin: format!("corpus {id} {name}"),
                name: name.clone(),
                schema: document_type
                    .schema()
                    .try_to_validating_json()
                    .expect("expected a JSON schema"),
                defs: defs.clone(),
                contract_id: contract.id(),
                config: contract.config().clone(),
                tokens: contract.tokens().clone(),
            });
        }
    }
    cases
}

// ---- generated schemas ----

/// Keys a generated schema gains: every keyword the meta-schema names somewhere, and some
/// it names nowhere.
const GENERATED_KEYS: [&str; 46] = [
    "type",
    "properties",
    "additionalProperties",
    "required",
    "indices",
    "position",
    "maxLength",
    "minLength",
    "pattern",
    "format",
    "items",
    "maxItems",
    "minItems",
    "uniqueItems",
    "byteArray",
    "contentMediaType",
    "enum",
    "const",
    "minimum",
    "maximum",
    "multipleOf",
    "refersTo",
    "encryptedFor",
    "distinctFrom",
    "maxBytes",
    "generatedFrom",
    "requiredSince",
    "$ref",
    "$id",
    "examples",
    "description",
    "documentsMutable",
    "canBeDeleted",
    "documentsSummable",
    "rangeAverageable",
    "moderatorAbilities",
    "actionFees",
    "tokenCost",
    "transient",
    "immutable",
    "name",
    "unique",
    "summableOffCountIndex",
    "rankedSummable",
    "unknownKeyword",
    "a-b",
];

fn generated_values() -> Vec<JsonValue> {
    vec![
        json!(null),
        json!(true),
        json!(false),
        json!(0),
        json!(1),
        json!(2),
        json!(-1),
        json!(1.5),
        json!(2.0),
        json!(32),
        json!(256),
        json!(65536),
        json!(4294967296_u64),
        json!(281474976710656_u64),
        json!(""),
        json!("a"),
        json!("a.b"),
        json!("a-b"),
        json!("asc"),
        json!("desc"),
        json!("object"),
        json!("array"),
        json!("string"),
        json!("integer"),
        json!("identity"),
        json!("deletableDocument"),
        json!("$ownerId"),
        json!("#/$defs/label"),
        json!("["),
        json!(IDENTIFIER_MEDIA_TYPE),
        json!("date-time"),
        json!("x".repeat(65)),
        json!("x".repeat(300)),
        json!([]),
        json!([1]),
        json!(["a"]),
        json!(["a", "a"]),
        json!([{ "a": "asc" }]),
        json!({}),
        json!({ "a": 1 }),
        json!({ "$id": "x" }),
        json!({ "at": "a" }),
        json!({ "type": "string", "maxLength": 10, "position": 9 }),
        json!({ "type": "identity" }),
        json!({ "owner": 1 }),
        json!({ "tokenPosition": 0, "amount": 1 }),
        json!({ "on": "$createdAt", "range": 1, "step": 1 }),
        json!({ "property": "a", "when": { "a": 1 } }),
    ]
}

/// The path of every value below the schema and the definitions of `case`.
fn paths(value: &JsonValue, path: &mut Vec<JsonValue>, found: &mut Vec<Vec<JsonValue>>) {
    found.push(path.clone());
    match value {
        JsonValue::Object(map) => {
            for (key, member) in map {
                path.push(json!(key));
                paths(member, path, found);
                path.pop();
            }
        }
        JsonValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(json!(index));
                paths(item, path, found);
                path.pop();
            }
        }
        _ => {}
    }
}

fn at_path<'v>(value: &'v mut JsonValue, path: &[JsonValue]) -> &'v mut JsonValue {
    path.iter().fold(value, |value, segment| match segment {
        JsonValue::String(key) => &mut value[key.as_str()],
        JsonValue::Number(index) => &mut value[index.as_u64().unwrap_or_default() as usize],
        _ => value,
    })
}

/// `case` with one to three changes at random places, one most often: a member removed, added, renamed,
/// nulled or replaced, an item removed, added, repeated or replaced.
fn mutated(case: &Case, rng: &mut StdRng, values: &[JsonValue]) -> Case {
    let mut mutated = case.clone();
    let mut target = json!({ "schema": mutated.schema, "defs": mutated.defs });
    let changes = match rng.gen_range(0..10) {
        0..=5 => 1,
        6..=8 => 2,
        _ => 3,
    };
    for _ in 0..changes {
        let mut found = Vec::new();
        paths(&target, &mut Vec::new(), &mut found);
        let candidates: Vec<&Vec<JsonValue>> = found
            .iter()
            .filter(|path| !path.is_empty() && (path[0] == json!("schema") || path.len() > 1))
            .collect();
        let path = candidates[rng.gen_range(0..candidates.len())].clone();
        let value = values[rng.gen_range(0..values.len())].clone();
        let node = at_path(&mut target, &path);
        match node {
            JsonValue::Object(map) if !map.is_empty() && rng.gen_bool(0.6) => {
                let key = map
                    .keys()
                    .nth(rng.gen_range(0..map.len()))
                    .cloned()
                    .unwrap_or_default();
                match rng.gen_range(0..4) {
                    0 => {
                        map.remove(&key);
                    }
                    1 => {
                        map.insert(key, JsonValue::Null);
                    }
                    2 => {
                        map.insert(key, value);
                    }
                    _ => {
                        if let Some(member) = map.remove(&key) {
                            let renamed = GENERATED_KEYS[rng.gen_range(0..GENERATED_KEYS.len())];
                            map.insert(renamed.to_string(), member);
                        }
                    }
                }
            }
            JsonValue::Object(map) => {
                let key = GENERATED_KEYS[rng.gen_range(0..GENERATED_KEYS.len())];
                map.insert(key.to_string(), value);
            }
            JsonValue::Array(items) if !items.is_empty() => {
                let index = rng.gen_range(0..items.len());
                match rng.gen_range(0..4) {
                    0 => {
                        items.remove(index);
                    }
                    1 => items.push(value),
                    2 => {
                        let item = items[index].clone();
                        items.push(item);
                    }
                    _ => items[index] = value,
                }
            }
            node => *node = value,
        }
    }
    mutated.schema = target["schema"].take();
    mutated.defs = match target["defs"].take() {
        JsonValue::Null => None,
        defs => Some(defs),
    };
    mutated.origin = format!("{} (generated)", case.origin);
    mutated
}

fn compare_all(cases: &[Case]) -> Tally {
    let mut tally = Tally::default();
    let differences: Vec<String> = cases
        .iter()
        .flat_map(|case| compare(case, &mut tally))
        .collect();
    assert!(
        differences.is_empty(),
        "{} differences, the first: {}",
        differences.len(),
        differences
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    tally
}

// ---- the tests ----

#[test]
fn should_admit_every_written_document_type_as_the_meta_schema_check_does() {
    let platform_version = PlatformVersion::latest();
    for case in written_cases() {
        let root = case
            .validating_root(platform_version)
            .unwrap_or_else(|| panic!("{}: expected a root", case.origin));
        assert_eq!(meta_schema_refusal(&root), None, "{}", case.origin);
        assert!(native_refusal(&root).is_none(), "{}", case.origin);
        let parsed = case.parse(true).expect("expected definitions");
        assert!(parsed.is_ok(), "{}: {parsed:?}", case.origin);
    }
}

#[test]
fn should_register_every_corpus_document_type_as_the_meta_schema_check_does() {
    let cases = corpus_cases();
    assert_eq!(cases.len(), 106 + 1397);
    let tally = compare_all(&cases);
    assert_eq!(tally.both_accept, cases.len(), "{tally:?}");
}

#[test]
fn should_refuse_generated_schemas_as_the_meta_schema_check_does() {
    let values = generated_values();
    let mut rng = StdRng::seed_from_u64(15);
    let written = written_cases();
    let cases: Vec<Case> = written
        .iter()
        .flat_map(|case| {
            (0..300)
                .map(|_| mutated(case, &mut rng, &values))
                .collect::<Vec<_>>()
        })
        .collect();
    let tally = compare_all(&cases);
    assert!(tally.both_refuse > 1000, "{tally:?}");
    assert!(tally.both_accept > 100, "{tally:?}");
}

#[test]
fn should_refuse_generated_corpus_schemas_as_the_meta_schema_check_does() {
    let values = generated_values();
    let mut rng = StdRng::seed_from_u64(1503);
    let cases: Vec<Case> = corpus_cases()
        .iter()
        .flat_map(|case| {
            (0..2)
                .map(|_| mutated(case, &mut rng, &values))
                .collect::<Vec<_>>()
        })
        .collect();
    let tally = compare_all(&cases);
    assert!(tally.both_refuse > 1000, "{tally:?}");
}

/// The root of the `profile` document type with `change` applied.
fn profile_root_with(change: impl FnOnce(&mut JsonValue)) -> JsonValue {
    let case = written_cases()
        .into_iter()
        .find(|case| case.name == "profile")
        .expect("expected the profile case");
    let mut root = case
        .validating_root(PlatformVersion::latest())
        .expect("expected a root");
    change(&mut root);
    root
}

/// A change to a document schema's root.
type RootChange = Box<dyn FnOnce(&mut JsonValue)>;

#[test]
fn should_refuse_each_meta_schema_rule_with_a_json_schema_error() {
    let cases: Vec<(&str, RootChange)> = vec![
        (
            "a null flag",
            Box::new(|root| root["documentsMutable"] = json!(null)),
        ),
        (
            "an unknown root key",
            Box::new(|root| root["documentMutable"] = json!(true)),
        ),
        (
            "a MASTER security level",
            Box::new(|root| root["signatureSecurityLevelRequirement"] = json!(0)),
        ),
        (
            "a dotted summed property",
            Box::new(|root| root["documentsSummable"] = json!("a.b")),
        ),
        (
            "a hyphenated property name",
            Box::new(|root| {
                root["properties"]["a-b"] =
                    json!({ "type": "string", "maxLength": 1, "position": 9 })
            }),
        ),
        (
            "an unknown property keyword",
            Box::new(|root| root["properties"]["name"]["nullable"] = json!(true)),
        ),
        (
            "a pattern without maxLength",
            Box::new(|root| {
                root["properties"]["note"]["pattern"] = json!("^a");
                root["properties"]["note"]
                    .as_object_mut()
                    .map(|note| note.remove("maxLength"));
            }),
        ),
        (
            "an identifier of 20 bytes",
            Box::new(|root| root["properties"]["avatar"]["minItems"] = json!(20)),
        ),
        (
            "items beside byteArray",
            Box::new(|root| root["properties"]["avatar"]["items"] = json!({ "type": "integer" })),
        ),
        (
            "an element keyword elements do not take",
            Box::new(|root| root["properties"]["tags"]["items"]["const"] = json!("a")),
        ),
        (
            "a nested property without position",
            Box::new(|root| {
                root["properties"]["meta"]["properties"]["a"]
                    .as_object_mut()
                    .map(|a| a.remove("position"));
            }),
        ),
        (
            "a duplicate enum member",
            Box::new(|root| root["properties"]["kind"]["enum"] = json!(["a", "a"])),
        ),
        (
            "a descending index property",
            Box::new(|root| root["indices"][1]["properties"][0]["score"] = json!("desc")),
        ),
        (
            "an index without a name",
            Box::new(|root| {
                root["indices"][1]
                    .as_object_mut()
                    .map(|index| index.remove("name"));
            }),
        ),
        (
            "a non-boolean unique",
            Box::new(|root| root["indices"][0]["unique"] = json!("yes")),
        ),
        (
            "an unknown refersTo key",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({ "type": "identity", "extra": 1 })
            }),
        ),
        (
            "a contract id beside an identity reference",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({
                    "type": "identity",
                    "contractId": "6hVQW16jyvZyGSQk2YVty4ND6bgFXozizYWnPt753uW5",
                })
            }),
        ),
        (
            "an empty definitions map",
            Box::new(|root| root["$defs"] = json!({})),
        ),
        (
            "a token amount past 48 bits",
            Box::new(|root| {
                root["tokenCost"] =
                    json!({ "create": { "tokenPosition": 0, "amount": 281474976710656_u64 } })
            }),
        ),
        (
            "a where `in` of 31 values",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({
                    "type": "permanentDocument",
                    "documentType": "post",
                    "findBy": { "author": "$ownerId" },
                    "where": { "kind": { "in": (0..31).collect::<Vec<_>>() } },
                })
            }),
        ),
    ];
    for (case, change) in cases {
        let root = profile_root_with(change);
        assert!(
            meta_schema_refusal(&root).is_some(),
            "{case}: the meta-schema admits it"
        );
        let refusal = native_refusal(&root).unwrap_or_else(|| panic!("{case}: admitted"));
        assert!(!is_index_property_name_cap(&refusal), "{case}");
    }
}

#[test]
fn should_report_where_and_which_rule_a_schema_breaks() {
    let root = profile_root_with(|root| {
        root["properties"]["meta"]["properties"]["a"]["maxLength"] = json!(-1)
    });
    let refusal = native_refusal(&root).expect("expected a refusal");
    assert_eq!(
        refusal.instance_path(),
        "/properties/meta/properties/a/maxLength"
    );
    assert_eq!(refusal.keyword(), "minimum");

    let root = profile_root_with(|root| {
        root["properties"]["meta"]["properties"]["a"]
            .as_object_mut()
            .map(|a| a.remove("position"));
    });
    let refusal = native_refusal(&root).expect("expected a refusal");
    assert_eq!(refusal.instance_path(), "/properties/meta/properties/a");
    assert_eq!(refusal.keyword(), "required");

    let root = profile_root_with(|root| root["properties"]["name"]["position"] = json!(null));
    let refusal = native_refusal(&root).expect("expected a refusal");
    assert_eq!(refusal.instance_path(), "/properties/name/position");
    assert_eq!(refusal.keyword(), "type");
}

#[test]
fn should_hold_index_property_names_to_255_bytes() {
    // 128 two-byte characters: 256 bytes, which the meta-schema's 256 characters admitted
    let too_long = "é".repeat(128);
    let longest = format!("{}a", "é".repeat(127));
    assert_eq!(longest.len(), 255);
    for (name, admitted) in [(too_long, false), (longest, true)] {
        let root = profile_root_with(|root| {
            let mut property = serde_json::Map::new();
            property.insert(name.clone(), json!("asc"));
            root["indices"][1]["properties"] = json!([property]);
        });
        assert_eq!(meta_schema_refusal(&root), None, "{name}");
        match native_refusal(&root) {
            None => assert!(admitted, "{} bytes admitted", name.len()),
            Some(refusal) => {
                assert!(!admitted, "{} bytes refused: {refusal}", name.len());
                assert!(is_index_property_name_cap(&refusal), "{refusal}");
                assert_eq!(refusal.instance_path(), "/indices/1/properties/0");
            }
        }
    }
}

#[test]
fn should_take_the_last_entry_of_a_repeated_key_as_the_meta_schema_did() {
    // The meta-schema check read a repeated key's last entry: so do the native checks
    let platform_version = PlatformVersion::latest();
    let mut case = written_cases()
        .into_iter()
        .find(|case| case.name == "profile")
        .expect("expected the profile case");
    case.schema["documentsMutable"] = json!(true);
    let mut schema = Value::from(case.schema.clone());
    if let Value::Map(map) = &mut schema {
        map.push((Value::Text("documentsMutable".to_string()), Value::Null));
    }
    let root = DocumentType::enrich_with_base_schema(schema, None, platform_version)
        .expect("expected a root")
        .try_to_validating_json()
        .expect("expected JSON");
    assert_eq!(root["documentsMutable"], JsonValue::Null);
    assert!(meta_schema_refusal(&root).is_some());
    assert!(native_refusal(&root).is_some());
}

#[test]
fn should_name_the_document_schema_uri_the_base_schema_adds() {
    let root = profile_root_with(|_| {});
    assert_eq!(root["$schema"], json!(DOCUMENT_SCHEMA_URI));
}

/// The first error of the meta-schema check on `root`, as that check returned it.
fn meta_schema_error(root: &JsonValue) -> Option<JsonSchemaError> {
    let meta_schema = select_document_meta_schema(3, "native schema tests")
        .expect("expected the document meta-schema");
    let error = meta_schema
        .validate(root)
        .err()
        .and_then(|mut errors| errors.next())
        .map(JsonSchemaError::from);
    error
}

#[test]
fn should_refuse_with_the_keyword_location_and_parameters_of_the_meta_schema_check() {
    let cases: Vec<(&str, RootChange)> = vec![
        (
            "a null flag",
            Box::new(|root| root["documentsMutable"] = json!(null)),
        ),
        (
            "an unknown root key",
            Box::new(|root| root["documentMutable"] = json!(true)),
        ),
        (
            "a MASTER security level",
            Box::new(|root| root["signatureSecurityLevelRequirement"] = json!(0)),
        ),
        (
            "a dotted summed property",
            Box::new(|root| root["documentsSummable"] = json!("a.b")),
        ),
        (
            "a hyphenated property name",
            Box::new(|root| {
                root["properties"]["a-b"] =
                    json!({ "type": "string", "maxLength": 1, "position": 9 })
            }),
        ),
        (
            "an unknown property keyword",
            Box::new(|root| root["properties"]["name"]["nullable"] = json!(true)),
        ),
        (
            "a pattern without maxLength",
            Box::new(|root| root["properties"]["score"]["pattern"] = json!("^a")),
        ),
        (
            "an identifier of 20 bytes",
            Box::new(|root| root["properties"]["avatar"]["minItems"] = json!(20)),
        ),
        (
            "items beside byteArray",
            Box::new(|root| root["properties"]["avatar"]["items"] = json!({ "type": "integer" })),
        ),
        (
            "an element keyword elements do not take",
            Box::new(|root| root["properties"]["tags"]["items"]["const"] = json!("a")),
        ),
        (
            "a nested property without position",
            Box::new(|root| {
                root["properties"]["meta"]["properties"]["a"]
                    .as_object_mut()
                    .map(|a| a.remove("position"));
            }),
        ),
        (
            "a duplicate enum member",
            Box::new(|root| root["properties"]["kind"]["enum"] = json!(["a", "a"])),
        ),
        (
            "a descending index property",
            Box::new(|root| root["indices"][1]["properties"][0]["score"] = json!("desc")),
        ),
        (
            "an index without a name",
            Box::new(|root| {
                root["indices"][1]
                    .as_object_mut()
                    .map(|index| index.remove("name"));
            }),
        ),
        (
            "a non-boolean unique",
            Box::new(|root| root["indices"][0]["unique"] = json!("yes")),
        ),
        (
            "an unknown refersTo key",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({ "type": "identity", "extra": 1 })
            }),
        ),
        (
            "a contract id beside an identity reference",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({
                    "type": "identity",
                    "contractId": "6hVQW16jyvZyGSQk2YVty4ND6bgFXozizYWnPt753uW5",
                })
            }),
        ),
        (
            "an empty definitions map",
            Box::new(|root| root["$defs"] = json!({})),
        ),
        (
            "a token amount past 48 bits",
            Box::new(|root| {
                root["tokenCost"] =
                    json!({ "create": { "tokenPosition": 0, "amount": 281474976710656_u64 } })
            }),
        ),
        (
            "a negative maxLength",
            Box::new(|root| root["properties"]["meta"]["properties"]["a"]["maxLength"] = json!(-1)),
        ),
        (
            "a zero multipleOf",
            Box::new(|root| root["properties"]["score"]["multipleOf"] = json!(0)),
        ),
        (
            "an unknown countable mode",
            Box::new(|root| root["indices"][0]["countable"] = json!("sometimes")),
        ),
        (
            "an uncompilable pattern",
            Box::new(|root| root["properties"]["name"]["pattern"] = json!("[")),
        ),
        (
            "a repeated type",
            Box::new(|root| root["properties"]["name"]["type"] = json!(["string", "string"])),
        ),
        (
            "an identityPublicKey reference without a key property",
            Box::new(|root| {
                root["properties"]["score"]["refersTo"] = json!({ "type": "identityPublicKey" })
            }),
        ),
        (
            "an element reference to an identity public key",
            Box::new(|root| {
                root["properties"]["tags"]["items"] = json!({
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": IDENTIFIER_MEDIA_TYPE,
                    "refersTo": { "type": "identityPublicKey", "keyIdProperty": "score" },
                })
            }),
        ),
        (
            "generatedFrom beside a $ref",
            Box::new(|root| {
                root["properties"]["label"]["type"] = json!("string");
                root["properties"]["label"]["generatedFrom"] = json!({
                    "function": "sys.stringTransformations.lowercase",
                    "params": ["name"],
                });
            }),
        ),
        (
            "a where value of no kind",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({
                    "type": "permanentDocument",
                    "documentType": "post",
                    "findBy": { "author": "$ownerId" },
                    "where": { "kind": 5 },
                })
            }),
        ),
        (
            "a terminal beside summableOffCountIndex",
            Box::new(|root| {
                root["indices"][0] = json!({
                    "name": "i",
                    "summableOffCountIndex": "base",
                    "rangeSummable": true,
                    "terminal": "name",
                })
            }),
        ),
        (
            "a rule name with a hyphen",
            Box::new(|root| root["propertyConstraints"] = json!({ "a-b": { "present": "name" } })),
        ),
        (
            "an equality of three operands",
            Box::new(|root| {
                root["propertyConstraints"] = json!({ "r": { "equal": ["score", 1, 2] } })
            }),
        ),
        (
            "an ifThen of three conditions",
            Box::new(|root| {
                let condition = json!({ "present": "name" });
                root["propertyConstraints"] =
                    json!({ "r": { "ifThen": [condition, condition, condition] } })
            }),
        ),
        (
            "a summed path that is no path",
            Box::new(|root| {
                root["propertyConstraints"] =
                    json!({ "r": { "lessThan": [{ "sumOf": ["post", "a-b"] }, 5] } })
            }),
        ),
        (
            "a counted filter key that is no path",
            Box::new(|root| {
                root["propertyConstraints"] =
                    json!({ "r": { "lessThan": [{ "countOf": ["post", { "a-b": "name" }] }, 5] } })
            }),
        ),
        (
            "an operand of no expression type",
            Box::new(|root| root["propertyConstraints"] = json!({ "r": { "equal": [1.5, 1] } })),
        ),
        (
            "an empty findBy key",
            Box::new(|root| {
                root["properties"]["avatar"]["refersTo"] = json!({
                    "type": "permanentDocument",
                    "documentType": "post",
                    "findBy": { "": "." },
                })
            }),
        ),
        (
            "a token contract id byte past 255",
            Box::new(|root| {
                let mut contract_id = vec![json!(1); 32];
                contract_id[0] = json!(256);
                root["tokenCost"] = json!({
                    "create": { "contractId": contract_id, "tokenPosition": 0, "amount": 1 },
                })
            }),
        ),
    ];
    let mut differences = Vec::new();
    for (case, change) in cases {
        let root = profile_root_with(change);
        let expected =
            meta_schema_error(&root).unwrap_or_else(|| panic!("{case}: the meta-schema admits it"));
        let refusal = native_refusal(&root).unwrap_or_else(|| panic!("{case}: admitted"));
        let shape = |error: &JsonSchemaError| {
            // A property name's error nests the name's own error in its parameters, which the
            // native checks give but for its location in the meta-schema
            let mut params = error.params().clone();
            if let Value::Map(params) = &mut params {
                params.retain(|(key, _)| key.as_text() != Some("schemaPath"));
            }
            (
                error.keyword().to_string(),
                error.instance_path().to_string(),
                params,
                error.property_name().to_string(),
            )
        };
        if shape(&refusal) != shape(&expected) {
            differences.push(format!(
                "{case}:\n  native {:?}\n  meta   {:?}",
                shape(&refusal),
                shape(&expected)
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

#[test]
fn should_refuse_a_type_whose_references_cycle_as_its_documents_would_be() {
    let defs = BTreeMap::from([
        ("a".to_string(), Value::from(json!({ "$ref": "#/$defs/b" }))),
        ("b".to_string(), Value::from(json!({ "$ref": "#/$defs/a" }))),
    ]);
    let schema = Value::from(json!({
        "type": "object",
        "properties": {
            "tags": {
                "type": "array",
                "position": 0,
                "maxItems": 4,
                "items": { "type": "integer" },
                "contains": { "$ref": "#/$defs/a" },
            },
        },
        "additionalProperties": false,
    }));
    let error = super::check_validation_schema(&schema, Some(&defs), PlatformVersion::latest())
        .expect_err("expected the type to be refused");
    assert!(
        matches!(
            &error,
            ProtocolError::ConsensusError(consensus) if matches!(
                consensus.as_ref(),
                ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(_)
                ))
            )
        ),
        "{error:?}"
    );
}

#[test]
fn should_compare_numbers_exactly_across_their_readings() {
    // Equal as f64, distinct as the meta-schema check compared them
    assert!(!super::equal_json(
        &json!(9007199254740993_u64),
        &json!(9007199254740992.0)
    ));
    assert!(super::equal_json(&json!(1_u64), &json!(1.0)));
    assert!(super::equal_json(&json!(-3_i64), &json!(-3.0)));
}

/// A `contains` that is not a schema, `"contains": 1`: the document meta-schema check does
/// not constrain `contains`, so the native checks admit it too. Registration still refuses
/// it under either check: when the JSON Schema validator compiles the schema, or when the
/// type's validation schema is built.
#[test]
fn should_admit_a_contains_that_is_no_schema_as_the_meta_schema_check_does() {
    let mut case = written_cases()
        .into_iter()
        .find(|case| case.name == "profile")
        .expect("expected the profile case");
    let position = case.schema["properties"]
        .as_object()
        .map_or(0, |properties| properties.len());
    case.schema["properties"]["tags"] = json!({
        "type": "array",
        "position": position,
        "maxItems": 2,
        "items": { "type": "integer" },
        "contains": 1,
    });
    let mut tally = Tally::default();
    let differences = compare(&case, &mut tally);
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(tally.both_accept, 1, "{tally:?}");
    assert_eq!(tally.parse_refusals, 1, "{tally:?}");
}

/// What the document meta-schema leaves unchecked, the native checks leave too: the value of
/// a property's `const`, which the meta-schema reads through the schema `true`, and the
/// schema of a system property, whose name the meta-schema admits without checking what it
/// names.
#[test]
fn should_admit_any_const_and_any_system_property_schema_as_the_meta_schema_check_does() {
    for (case, change) in [
        (
            "a number const",
            Box::new(|root: &mut JsonValue| {
                root["properties"]["score"]["const"] = json!(10);
            }) as RootChange,
        ),
        (
            "an unknown keyword on a system property schema",
            Box::new(|root: &mut JsonValue| {
                root["properties"]["$createdAt"] =
                    json!({ "type": "integer", "unknownKeyword": 1 });
            }) as RootChange,
        ),
        (
            "a system property schema with a malformed bound",
            Box::new(|root: &mut JsonValue| {
                root["properties"]["$createdAt"] = json!({ "type": "integer", "minimum": "a" });
            }) as RootChange,
        ),
    ] {
        let root = profile_root_with(change);
        assert_eq!(
            meta_schema_refusal(&root),
            None,
            "{case}: the meta-schema check"
        );
        assert!(native_refusal(&root).is_none(), "{case}: the native checks");
    }
}

/// A `tokenCost` `contractId` holding a float with no fractional part: the meta-schema checks
/// the bytes with `byteArray`, which reads each as an unsigned integer and refuses a float, so
/// the native checks refuse it too.
#[test]
fn should_refuse_a_float_token_cost_contract_id_byte_as_the_meta_schema_check_does() {
    let contract_id = |first: JsonValue| {
        let mut bytes = vec![json!(0); 32];
        bytes[0] = first;
        JsonValue::Array(bytes)
    };
    for (first, refused) in [(json!(1), false), (json!(1.0), true), (json!(256), true)] {
        let root = profile_root_with(|root| {
            root["tokenCost"] = json!({
                "create": { "contractId": contract_id(first.clone()), "tokenPosition": 0, "amount": 1 }
            });
        });
        assert_eq!(
            meta_schema_refusal(&root).is_some(),
            refused,
            "{first}: the meta-schema check"
        );
        assert_eq!(
            native_refusal(&root).is_some(),
            refused,
            "{first}: the native checks"
        );
    }
}

/// A rule of protocol version 15: in a list that must be unique, such as an `enum`'s members,
/// `0.0` and `-0.0` are one value at every length, as the native document validator compares
/// them.
#[test]
fn should_hold_zero_and_negative_zero_as_one_enum_member_at_every_length() {
    // The two numbers given, then distinct integers up to `length` members
    let members = |length: usize, first: f64, second: f64| {
        let mut members = vec![json!(first), json!(second)];
        members.extend((1..=length - 2).map(|member| json!(member)));
        JsonValue::Array(members)
    };
    for length in [2, 15, 16, 17] {
        let root = profile_root_with(|root| {
            root["properties"]["score"]["enum"] = members(length, 0.0, -0.0);
        });
        assert_eq!(
            native_refusal(&root).map(|error| error.keyword().to_string()),
            Some("uniqueItems".to_string()),
            "{length} members holding 0.0 and -0.0"
        );
        let root = profile_root_with(|root| {
            root["properties"]["score"]["enum"] = members(length, 0.0, 0.5);
        });
        assert!(native_refusal(&root).is_none(), "{length} distinct members");
    }
}
