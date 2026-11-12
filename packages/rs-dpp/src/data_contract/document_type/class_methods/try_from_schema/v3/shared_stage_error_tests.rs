//! The class of error a contract refused in a shared stage is reported with.
//!
//! The doctype-level aggregate stages are shared with generation 2, and the
//! core parse with generations 1 and 2. Generation 3 reports what they refuse
//! as a consensus error, so a transition carrying the contract is a paid
//! rejection; the earlier generations keep the bare
//! `ProtocolError::DataContractError` or `ProtocolError::ValueError` they
//! shipped with, which a node refuses unpaid.

use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use assert_matches::assert_matches;
use platform_value::platform_value;

/// Two bounded integers to sum, one integer that parses as `u64` (a `minimum`
/// and no `maximum`), one bounded integer left out of `required`, and a string.
fn schema_with_doctype_keys(keys: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "amount": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 0},
            "fee": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 1},
            "unbounded": {"type": "integer", "minimum": 1, "position": 2},
            "optional": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 3},
            "label": {"type": "string", "maxLength": 20, "position": 4},
        },
        "required": ["amount", "fee", "unbounded"],
        "additionalProperties": false,
    });
    for (key, value) in keys.into_map().expect("the doctype keys are a map") {
        schema
            .set_value(key.as_text().expect("a doctype key is text"), value)
            .expect("the doctype key applies");
    }
    schema
}

/// Every rule the two aggregate stages enforce, with a fragment of its message.
fn broken_aggregate_rules() -> Vec<(Value, &'static str)> {
    vec![
        // `parse_doctype_aggregate_keywords`
        (
            platform_value!({"documentsSummable": ""}),
            "documentsSummable must be a non-empty string",
        ),
        (
            platform_value!({"documentsSummable": 5}),
            "documentsSummable value must be a string or null",
        ),
        (
            platform_value!({"documentsAverageable": ""}),
            "documentsAverageable must be a non-empty string",
        ),
        (
            platform_value!({"documentsAverageable": 5}),
            "documentsAverageable value must be a string or null",
        ),
        (
            platform_value!({"documentsAverageable": "amount", "documentsSummable": "fee"}),
            "conflicts with documentsSummable",
        ),
        (
            platform_value!({"documentsAverageable": "amount", "documentsCountable": false}),
            "explicitly sets documentsCountable: false",
        ),
        (
            platform_value!({
                "documentsAverageable": "amount",
                "rangeAverageable": true,
                "rangeCountable": false,
            }),
            "conflicts with explicit rangeCountable: false",
        ),
        (
            platform_value!({
                "documentsAverageable": "amount",
                "rangeAverageable": true,
                "rangeSummable": false,
            }),
            "conflicts with explicit rangeSummable: false",
        ),
        (
            platform_value!({"rangeAverageable": true}),
            "requires documentsAverageable",
        ),
        (
            platform_value!({"rangeSummable": true}),
            "rangeSummable: true requires documentsSummable",
        ),
        // `apply_doctype_aggregates`
        (
            platform_value!({
                "documentsSummable": "amount",
                "indices": [
                    {"name": "byLabel", "properties": [{"label": "asc"}], "summable": "fee"},
                ],
            }),
            "must name the same property",
        ),
        (
            platform_value!({"documentsSummable": "missing"}),
            "does not exist on that document type",
        ),
        (
            platform_value!({"documentsSummable": "unbounded"}),
            "whose values fit in i64",
        ),
        (
            platform_value!({"documentsSummable": "optional"}),
            "listed in the document type's `required` array",
        ),
    ]
}

fn parse_dispatched(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "payment",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

#[test]
fn should_report_every_broken_aggregate_rule_as_a_consensus_error() {
    for full_validation in [false, true] {
        for (keys, needle) in broken_aggregate_rules() {
            let result = parse_dispatched(
                schema_with_doctype_keys(keys),
                PlatformVersion::latest(),
                full_validation,
            );
            match result {
                Err(ProtocolError::ConsensusError(error)) => match *error {
                    ConsensusError::BasicError(BasicError::ContractError(error)) => assert!(
                        error.to_string().contains(needle),
                        "full_validation={full_validation}: expected {needle:?}, got: {error}"
                    ),
                    other => panic!(
                        "full_validation={full_validation}: expected a contract error \
                         containing {needle:?}, got {other}"
                    ),
                },
                other => panic!(
                    "full_validation={full_validation}: expected a consensus error \
                     containing {needle:?}, got {other:?}"
                ),
            }
        }
    }
}

/// Generation 2 is left as it shipped: a node running this code at protocol
/// version 13 has to refuse such a transition exactly as the release that
/// shipped protocol version 13 does.
#[test]
fn should_keep_reporting_broken_aggregate_rules_as_bare_errors_at_protocol_version_13() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
    for (keys, needle) in broken_aggregate_rules() {
        match parse_dispatched(schema_with_doctype_keys(keys), platform_version, false) {
            Err(ProtocolError::DataContractError(error)) => assert!(
                error.to_string().contains(needle),
                "expected {needle:?}, got: {error}"
            ),
            other => panic!("expected a bare error containing {needle:?}, got {other:?}"),
        }
    }
}

/// Schema values of the wrong shape the core parse reads: a `position` past
/// `u32`, read under full validation after the meta-schema admitted it, and a
/// `tokenCost` amount that is no integer, read on the non-validating path,
/// where no meta-schema runs first.
fn malformed_core_values() -> Vec<(&'static str, Value, bool)> {
    vec![
        (
            "a position past u32",
            platform_value!({
                "type": "object",
                "properties": {
                    "amount": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 1000,
                        "position": 4294967296u64,
                    },
                },
                "required": ["amount"],
                "additionalProperties": false,
            }),
            true,
        ),
        (
            "a tokenCost amount that is no integer",
            schema_with_doctype_keys(platform_value!({
                "tokenCost": {"create": {"tokenPosition": 0, "amount": true}},
            })),
            false,
        ),
    ]
}

#[test]
fn should_report_a_malformed_core_value_as_a_consensus_error() {
    for (what, schema, full_validation) in malformed_core_values() {
        match parse_dispatched(schema, PlatformVersion::latest(), full_validation) {
            Err(ProtocolError::ConsensusError(error)) => assert_matches!(
                *error,
                ConsensusError::BasicError(BasicError::ValueError(_)),
                "{what}"
            ),
            other => panic!("{what}: expected a consensus value error, got {other:?}"),
        }
    }
}

/// The core parse is left as it shipped for generations 1 and 2.
#[test]
fn should_keep_reporting_a_malformed_core_value_as_a_bare_error_at_protocol_version_13() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
    for (what, schema, full_validation) in malformed_core_values() {
        assert_matches!(
            parse_dispatched(schema, platform_version, full_validation),
            Err(ProtocolError::ValueError(_)),
            "{what}"
        );
    }
}
