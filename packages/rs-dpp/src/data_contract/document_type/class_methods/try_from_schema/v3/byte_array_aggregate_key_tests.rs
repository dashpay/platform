//! `countOf` and `sumOf` matching their documents by a byte array key, bound to a byte array
//! property of the document written (`parse_property_constraints` 1, protocol version 15).
//! Protocol version 14 parses the same rule as reading the byte array as an integer, and
//! refuses it.

use super::immutable_tests::expect_structure_error;
use super::*;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    AggregateBinding, AggregateKind, AggregateRead, EqualityKind, PropertyRead,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use serde_json::json;

/// A contract of one type, `commitment`, holding a 32-byte `nameHash`, a 20-byte `shortHash`
/// and an identifier `ownerRef`, each kept counted by an index of its own, and declaring
/// `rules` under `keyword` (`propertyConstraints` or `deleteConstraints`), parsed under
/// `platform_version`.
fn commitments(
    keyword: &str,
    rules: serde_json::Value,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let mut commitment = json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "nameHash": { "type": "bytes", "size": 32, "position": 0 },
            "shortHash": { "type": "bytes", "size": 20, "position": 1 },
            "ownerRef": { "type": "identifier", "position": 2 }
        },
        "required": ["nameHash", "shortHash", "ownerRef"],
        "indices": [
            {
                "name": "byNameHash",
                "properties": [{ "nameHash": "asc" }],
                "countable": "countable"
            },
            {
                "name": "byOwnerRef",
                "properties": [{ "ownerRef": "asc" }],
                "countable": "countable"
            }
        ],
        "additionalProperties": false
    });
    commitment[keyword] = rules;
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": { "commitment": commitment }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        true,
        platform_version,
    )
}

/// At most two commitments of a name, counted by the byte array they hold.
fn at_most_two_per_name() -> serde_json::Value {
    json!({
        "atMostTwoPerName": {
            "lessThanOrEqual": [{ "countOf": ["commitment", { "nameHash": "nameHash" }] }, 2]
        }
    })
}

fn by_name_hash() -> AggregateRead {
    AggregateRead {
        kind: AggregateKind::Count,
        document_type: "commitment".to_string(),
        filter: [(
            "nameHash".to_string(),
            AggregateBinding::Property {
                path: "nameHash".to_string(),
                kind: Some(EqualityKind::Bytes),
            },
        )]
        .into(),
        of_own_type: true,
    }
}

/// From protocol version 15 a `countOf` matches its documents by a byte array key bound to a
/// byte array property, and reads that property as the byte array it is; protocol version 14
/// reads it as an integer, which a byte array is not.
#[test]
fn should_count_by_a_byte_array_key_from_protocol_version_15() {
    let contract = commitments(
        "propertyConstraints",
        at_most_two_per_name(),
        PlatformVersion::latest(),
    )
    .expect("a byte array key bound to a byte array registers");
    let commitment = contract
        .document_type_for_name("commitment")
        .expect("commitment");
    let rule = &commitment.property_constraints()["atMostTwoPerName"];
    assert_eq!(rule.aggregate_reads(), [&by_name_hash()]);
    assert_eq!(
        rule.property_reads(),
        [("nameHash", PropertyRead::Bytes { extent: 0 })]
    );
    assert_eq!(
        by_name_hash()
            .answering_index(&commitment)
            .map(|index| index.name.as_str()),
        Some("byNameHash")
    );

    expect_structure_error(
        commitments(
            "propertyConstraints",
            at_most_two_per_name(),
            PlatformVersion::get(14).expect("protocol version 14"),
        ),
        "rule \"atMostTwoPerName\" reads \"nameHash\", which has type byteArray, not integer \
         or boolean",
    );
}

/// A `deleteConstraints` rule parses with the same grammar, byte array keys included.
#[test]
fn should_count_by_a_byte_array_key_in_a_delete_rule() {
    let contract = commitments(
        "deleteConstraints",
        json!({
            "notTheLastOfItsName": {
                "greaterThan": [{ "countOf": ["commitment", { "nameHash": "nameHash" }] }, 0]
            }
        }),
        PlatformVersion::latest(),
    )
    .expect("a delete rule may count by a byte array key");
    let commitment = contract
        .document_type_for_name("commitment")
        .expect("commitment");
    assert_eq!(
        commitment.delete_constraints()["notTheLastOfItsName"].aggregate_reads(),
        [&by_name_hash()]
    );
}

/// A byte array key is matched by a byte array property alone, and a byte array property
/// matches no key of another kind.
#[test]
fn should_refuse_a_byte_array_key_matched_by_another_kind() {
    let refused = |filter: serde_json::Value, needle: &str| {
        expect_structure_error(
            commitments(
                "propertyConstraints",
                json!({
                    "rule": {
                        "lessThanOrEqual": [{ "countOf": ["commitment", filter] }, 2]
                    }
                }),
                PlatformVersion::latest(),
            ),
            needle,
        );
    };
    refused(
        json!({ "nameHash": "$ownerId" }),
        "counts \"commitment\" with \"nameHash\", a byte array, at $ownerId, an identifier",
    );
    refused(
        json!({ "nameHash": "$id" }),
        "counts \"commitment\" with \"nameHash\", a byte array, at $id, an identifier",
    );
    refused(
        json!({ "nameHash": "ownerRef" }),
        "counts \"commitment\" with \"nameHash\", a byte array, at \"ownerRef\", an identifier",
    );
    refused(
        json!({ "ownerRef": "nameHash" }),
        "counts \"commitment\" with \"ownerRef\", an identifier, at \"nameHash\", a byte array",
    );
    refused(
        json!({ "nameHash": { "const": "00" } }),
        "counts \"commitment\" with \"nameHash\" at the constant \"00\", but \"nameHash\" is a \
         byte array, matched by a byte array property of the document",
    );
}

/// The total is read from the count tree of the byte array the document holds: the filter
/// takes that byte array as the key of the counted type holds it, and a document of the type
/// adds to the total only when it holds the same one.
#[test]
fn should_match_a_document_by_the_byte_array_it_holds() {
    let platform_version = PlatformVersion::latest();
    let contract = commitments(
        "propertyConstraints",
        at_most_two_per_name(),
        platform_version,
    )
    .expect("the contract registers");
    let commitment = contract
        .document_type_for_name("commitment")
        .expect("commitment");
    let owner = Identifier::from([3; 32]);
    let document = |name_hash: [u8; 32]| {
        Value::from(std::collections::BTreeMap::from([
            ("nameHash".to_string(), Value::Bytes32(name_hash)),
            ("shortHash".to_string(), Value::Bytes20([9; 20])),
            ("ownerRef".to_string(), Value::Identifier([4; 32])),
        ]))
    };
    let read = by_name_hash();
    let values = read
        .filter_values(
            commitment,
            Identifier::from([5; 32]),
            &document([1; 32]),
            owner,
            platform_version,
        )
        .expect("reads")
        .expect("a byte array the key holds");
    assert_eq!(values, [("nameHash".to_string(), Value::Bytes32([1; 32]))]);
    assert_eq!(
        read.contribution(
            commitment,
            &values,
            &document([1; 32]),
            owner,
            platform_version
        )
        .expect("reads"),
        1
    );
    assert_eq!(
        read.contribution(
            commitment,
            &values,
            &document([2; 32]),
            owner,
            platform_version
        )
        .expect("reads"),
        0
    );
}
