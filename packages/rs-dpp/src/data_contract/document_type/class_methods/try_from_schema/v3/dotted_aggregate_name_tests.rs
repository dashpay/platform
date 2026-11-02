//! The property an aggregate keyword names is a top-level one.
//!
//! `summable` and `averageable` on an index, and `documentsSummable` and
//! `documentsAverageable` on the document type, name the integer property each
//! document contributes to the sum. Drive reads that value from the top level
//! of the document, while the parser resolves the name in the flattened
//! properties and the required fields, which also hold the dotted path of a
//! property nested in an object. A dotted name therefore registered, and then
//! every document create of the type failed. Meta-schema v3 (protocol version
//! 14) refuses the dot at registration. Meta-schema v2 (protocol version 13)
//! keeps admitting it, and a stored contract, parsed without full validation,
//! keeps loading.

use super::immutable_tests::parse_dispatched;
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::string_encoding::Encoding;
use serde_json::json;

/// The dotted path of `amount`, an integer in the `payment` object.
const DOTTED: &str = "payment.amount";

/// A document type with a required `payment` object holding a required integer
/// `amount`, a string `label` to index, and `keys` set at its top level.
fn schema_with(keys: serde_json::Value) -> serde_json::Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "payment": {
                "type": "object",
                "properties": {
                    "amount": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 0}
                },
                "required": ["amount"],
                "additionalProperties": false,
                "position": 0
            },
            "label": {"type": "string", "maxLength": 20, "position": 1}
        },
        "required": ["payment", "label"],
        "additionalProperties": false
    });
    for (key, value) in keys.as_object().expect("the keys are an object") {
        schema[key] = value.clone();
    }
    schema
}

/// Each aggregate keyword naming `name`, with the path of the keyword in the
/// document type schema.
fn each_aggregate_keyword(name: &str) -> Vec<(serde_json::Value, &'static str)> {
    let index_with = |keyword: &str| {
        json!({
            "indices": [
                {"name": "byLabel", "properties": [{"label": "asc"}], keyword: name}
            ]
        })
    };
    vec![
        (index_with("summable"), "/indices/0/summable"),
        (index_with("averageable"), "/indices/0/averageable"),
        (json!({ "documentsSummable": name }), "/documentsSummable"),
        (
            json!({ "documentsAverageable": name }),
            "/documentsAverageable",
        ),
    ]
}

fn to_value(schema: serde_json::Value) -> Value {
    platform_value::to_value(schema).expect("the schema converts")
}

/// The name each keyword resolved to on a parsed document type.
fn aggregate_name(
    document_type: &(impl DocumentTypeV0Getters + DocumentTypeV2Getters),
) -> Option<String> {
    document_type
        .documents_summable()
        .map(str::to_string)
        .or_else(|| {
            document_type
                .indexes()
                .values()
                .find_map(|index| index.summable.clone())
        })
}

#[test]
fn should_refuse_a_dotted_name_in_each_aggregate_keyword_at_registration() {
    for (keys, path) in each_aggregate_keyword(DOTTED) {
        let result = parse_dispatched(to_value(schema_with(keys)), PlatformVersion::latest(), true);
        match result {
            Err(ProtocolError::ConsensusError(error)) => match *error {
                ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => {
                    assert_eq!(error.keyword(), "pattern", "{path}: {error}");
                    assert_eq!(error.instance_path(), path, "{error}");
                }
                other => panic!("{path}: expected a JSON schema error, got {other}"),
            },
            other => panic!("{path}: expected a consensus error, got {other:?}"),
        }
    }
}

#[test]
fn should_accept_a_top_level_name_in_each_aggregate_keyword_at_registration() {
    for (keys, path) in each_aggregate_keyword("total") {
        let mut schema = schema_with(keys);
        schema["properties"]["total"] =
            json!({"type": "integer", "minimum": 0, "maximum": 1000, "position": 2});
        schema["required"] = json!(["payment", "label", "total"]);

        let document_type = parse_dispatched(to_value(schema), PlatformVersion::latest(), true)
            .unwrap_or_else(|error| panic!("{path}: {error:?}"));
        assert_eq!(aggregate_name(&document_type).as_deref(), Some("total"));
    }
}

/// Meta-schema v2 shipped bounding only the name's length, and it keeps doing
/// so for every block of protocol version 13.
#[test]
fn should_keep_accepting_a_dotted_name_at_protocol_version_13() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
    for (keys, path) in each_aggregate_keyword(DOTTED) {
        let document_type = parse_dispatched(to_value(schema_with(keys)), platform_version, true)
            .unwrap_or_else(|error| panic!("{path}: {error:?}"));
        assert_eq!(aggregate_name(&document_type).as_deref(), Some(DOTTED));
    }
}

/// Drive reads a stored contract without full validation, so the meta-schema
/// does not run on it, and a contract registered at protocol version 13 with a
/// dotted name keeps loading at protocol version 14.
#[test]
fn should_load_a_contract_registered_at_protocol_version_13_with_a_dotted_name_at_protocol_version_14(
) {
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13 exists");
    for (keys, path) in each_aggregate_keyword(DOTTED) {
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": { "payment": schema_with(keys) }
        });
        let registered = DataContract::from_value(to_value(contract), true, platform_version_13)
            .unwrap_or_else(|error| panic!("{path}: {error:?}"));
        let stored = registered
            .serialize_to_bytes_with_platform_version(platform_version_13)
            .expect("the contract serializes");

        let loaded =
            DataContract::versioned_deserialize_trusted(&stored, false, PlatformVersion::latest())
                .unwrap_or_else(|error| panic!("{path}: {error:?}"));
        let document_type = loaded
            .document_type_for_name("payment")
            .expect("the payment type");
        assert_eq!(
            aggregate_name(&document_type).as_deref(),
            Some(DOTTED),
            "{path}"
        );
    }
}
