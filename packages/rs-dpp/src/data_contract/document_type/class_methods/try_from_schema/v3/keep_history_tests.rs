//! The keep-history document lifecycle as generation 3 admits it: a
//! keep-history type may allow deletion, may additionally allow erasure, and
//! may not carry a contested index.
use super::*;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::errors::DataContractError;
use platform_value::platform_value;

/// Parses through the public dispatcher at the given protocol version so
/// the test exercises the same `try_from_schema` version routing consensus
/// code uses (v3 at protocol version 14, v2 at 12 and 13).
fn parse_at_version(
    schema: Value,
    protocol_version: u32,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "test_doc",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_at_version(schema, 14, true)
}

fn keep_history_deletable_schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": {
                "type": "string",
                "maxLength": 50,
                "position": 0,
            },
        },
        "additionalProperties": false,
        "documentsKeepHistory": true,
        "canBeDeleted": true,
    })
}

/// The two settings describe different things and no longer contradict each
/// other: history decides what is retained, deletion decides what ordinary
/// reads can still see. A delete on such a type removes the document from
/// every ordinary read and leaves its revisions readable.
#[test]
fn should_accept_a_keep_history_type_that_allows_deletion() {
    let document_type = parse(keep_history_deletable_schema())
        .expect("a keep-history type may allow deletion from protocol version 14");
    assert!(document_type.documents_keep_history());
    assert!(document_type.documents_can_be_deleted());
    assert!(
        !document_type.documents_can_be_erased(),
        "erasure is not implied by deletion; it has to be asked for"
    );
}

/// The contract config defaults `canBeDeleted` to true, so a keep-history type
/// that says nothing about deletion is deletable — the same value it carried
/// before protocol 14, now with a delete that works.
#[test]
fn should_accept_a_keep_history_type_that_omits_the_delete_flag() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "additionalProperties": false,
        "documentsKeepHistory": true,
    });
    let document_type = parse(schema).expect("the omitted flag defaults to deletable");
    assert!(document_type.documents_keep_history());
    assert!(document_type.documents_can_be_deleted());
}

/// A keep-history type that withholds deletion is still valid: its documents
/// are append-only and can never leave ordinary reads.
#[test]
fn should_accept_a_keep_history_type_that_withholds_deletion() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "additionalProperties": false,
        "documentsKeepHistory": true,
        "canBeDeleted": false,
    });
    let document_type = parse(schema).expect("an append-only keep-history type is valid");
    assert!(document_type.documents_keep_history());
    assert!(!document_type.documents_can_be_deleted());
}

/// The released parsers never saw the rule and must keep parsing the same
/// schemas the same way, so a historical block replays identically.
#[test]
fn should_accept_a_keep_history_deletable_type_at_released_protocol_versions() {
    for protocol in [12, 13] {
        let document_type = parse_at_version(keep_history_deletable_schema(), protocol, true)
            .unwrap_or_else(|error| {
                panic!("protocol {protocol} must still accept the schema: {error:?}")
            });
        assert!(document_type.documents_keep_history());
        assert!(document_type.documents_can_be_deleted());
    }
}

fn erasable_schema(keep_history: bool, can_be_deleted: bool, can_be_erased: bool) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "additionalProperties": false,
        "documentsKeepHistory": keep_history,
        "canBeDeleted": can_be_deleted,
        "canBeErased": can_be_erased,
    })
}

#[test]
fn should_accept_an_erasable_keep_history_deletable_type() {
    let document_type =
        parse(erasable_schema(true, true, true)).expect("the admitted combination must parse");
    assert!(document_type.documents_can_be_erased());
}

/// Erasure purges retained revisions of a document that has already been
/// deleted, so a type with no history has nothing to purge and a type whose
/// documents can never be deleted can never reach the state erasure acts on.
/// Neither is silently ignored.
#[test]
fn should_reject_erasure_without_history_or_without_deletion() {
    for (keep_history, can_be_deleted) in [(false, true), (true, false), (false, false)] {
        let error = parse(erasable_schema(keep_history, can_be_deleted, true)).unwrap_err();
        let message = format!("{error:?}");
        assert!(
            message.contains("canBeErased"),
            "the error must name the flag the author has to change; got {message}"
        );
        assert_contract_structure_consensus_error(&error);
    }
}

/// A signed contract carrying a refused combination must be a paid rejection
/// with a nonce bump, and only the consensus variant becomes one: the bare
/// data-contract variant escapes the transformation as an internal execution
/// error, which costs the submitter nothing and reports nothing useful.
#[cfg(feature = "validation")]
fn assert_contract_structure_consensus_error(error: &ProtocolError) {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    let ProtocolError::ConsensusError(consensus) = error else {
        panic!("expected a consensus error, got {error:?}");
    };
    assert!(
        matches!(
            consensus.as_ref(),
            ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(_)
            ))
        ),
        "expected an invalid-contract-structure basic error, got {consensus:?}"
    );
}

#[cfg(not(feature = "validation"))]
fn assert_contract_structure_consensus_error(error: &ProtocolError) {
    assert!(
        matches!(
            error,
            ProtocolError::DataContractError(DataContractError::InvalidContractStructure(_))
        ),
        "without the validation feature the structural variant is all there is, got {error:?}"
    );
}

/// Erasure defaults to off: a type that says nothing about it cannot have its
/// revisions purged.
#[test]
fn should_default_erasure_to_off() {
    let document_type = parse(keep_history_deletable_schema()).expect("parses");
    assert!(!document_type.documents_can_be_erased());
}

/// The keyword does not exist in the earlier meta-schemas, so a contract that
/// declares it is refused there rather than silently parsing without it.
#[test]
fn should_reject_the_erasure_keyword_at_released_protocol_versions() {
    for protocol in [12, 13] {
        assert!(
            parse_at_version(erasable_schema(true, true, true), protocol, true).is_err(),
            "protocol {protocol} has no canBeErased keyword"
        );
    }
}

/// A contested resource is awarded outside transition validation, at an id
/// derived from the winner rather than from the contested values, so that award
/// can land on an id whose retained history already exists. The two are kept
/// apart at registration until the contested machinery can handle it.
#[test]
fn should_reject_a_keep_history_type_that_carries_a_contested_index() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "indices": [
            {
                "name": "byLabel",
                "properties": [{"label": "asc"}],
                "unique": true,
                "contested": {
                    "fieldMatches": [{"field": "label", "regexPattern": "^[a-z]{3,10}$"}],
                    "resolution": 0,
                },
            },
        ],
        "required": ["label"],
        "additionalProperties": false,
        "documentsMutable": false,
        "documentsKeepHistory": true,
        "canBeDeleted": false,
    });
    let error = parse(schema).expect_err("a contested keep-history type must be refused");
    let message = format!("{error:?}");
    assert!(
        message.contains("contested"),
        "the error must say which index is the problem; got {message}"
    );
    assert_contract_structure_consensus_error(&error);
}

/// The same index without history is unaffected: the refusal is about the
/// combination, not about contested indexes.
#[test]
fn should_accept_a_contested_index_without_history() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "indices": [
            {
                "name": "byLabel",
                "properties": [{"label": "asc"}],
                "unique": true,
                "contested": {
                    "fieldMatches": [{"field": "label", "regexPattern": "^[a-z]{3,10}$"}],
                    "resolution": 0,
                },
            },
        ],
        "required": ["label"],
        "additionalProperties": false,
        "documentsMutable": false,
    });
    parse(schema).expect("a contested index alone is fine");
}

fn repair_schema(keep_history: bool, can_be_deleted: bool) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "additionalProperties": false,
        "documentsKeepHistory": keep_history,
        "canBeDeleted": can_be_deleted,
    })
}

/// A keep-history type may withdraw deletion, and only in that direction.
#[test]
fn should_allow_a_keep_history_type_to_withdraw_deletion_at_protocol_14() {
    let old = parse_at_version(repair_schema(true, true), 13, true).unwrap();
    let new = parse_at_version(repair_schema(true, false), 14, true).unwrap();
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
        .expect("the update must reach a consensus result");
    assert!(result.is_valid(), "rejected: {:?}", result.errors);
}

#[test]
fn should_preserve_the_immutable_delete_flag_through_protocol_13() {
    for protocol in [12, 13] {
        let old = parse_at_version(repair_schema(true, true), protocol, true).unwrap();
        let new = parse_at_version(repair_schema(true, false), protocol, true).unwrap();
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, PlatformVersion::get(protocol).unwrap())
            .unwrap();
        assert!(
            !result.is_valid(),
            "protocol {protocol} must still refuse the change"
        );
    }
}

#[test]
fn should_reject_other_delete_and_history_flag_changes_at_protocol_14() {
    for (old_flags, new_flags) in [
        ((false, true), (false, false)),
        ((false, false), (false, true)),
        ((true, false), (true, true)),
        ((true, true), (false, false)),
        ((false, true), (true, false)),
    ] {
        let old = parse_at_version(repair_schema(old_flags.0, old_flags.1), 13, true).unwrap();
        // A caller may already have a parsed contract; update validation must
        // enforce immutability even without the full-validation parser guard.
        let new = parse_at_version(repair_schema(new_flags.0, new_flags.1), 14, false).unwrap();
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
            .unwrap();
        assert!(
            !result.is_valid(),
            "unexpectedly accepted {old_flags:?} -> {new_flags:?}"
        );
    }
}

/// Erasability is immutable in both directions. Widening it hands an
/// irreversible operation to a type registered without it; narrowing it after a
/// first chunk has removed revisions strands a partially erased document.
#[test]
fn should_reject_every_change_to_the_erasure_flag() {
    for (before, after) in [(false, true), (true, false)] {
        let old = parse_at_version(erasable_schema(true, true, before), 14, false).unwrap();
        let new = parse_at_version(erasable_schema(true, true, after), 14, false).unwrap();
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
            .unwrap();
        assert!(
            !result.is_valid(),
            "unexpectedly accepted canBeErased {before} -> {after}"
        );
    }
}

/// An erasable type can never stop being deletable, whichever way the update is
/// spelled: erasure applies to deleted documents only, and since erasability
/// itself cannot be withdrawn, such a type could never reach a state erasure
/// acts on again. Keeping the flag makes the type unparseable; dropping it
/// changes an immutable flag.
#[test]
fn should_refuse_to_withdraw_deletion_from_an_erasable_type() {
    assert!(
        parse_at_version(erasable_schema(true, false, true), 14, false).is_err(),
        "an erasable type that forbids deletion is not a valid type at all"
    );

    let old = parse_at_version(erasable_schema(true, true, true), 14, false).unwrap();
    let new = parse_at_version(repair_schema(true, false), 14, false).unwrap();
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
        .unwrap();
    assert!(!result.is_valid(), "an erasable type kept its delete flag");
}

#[test]
fn should_reject_incompatible_properties_during_a_delete_flag_withdrawal() {
    let old = parse_at_version(repair_schema(true, true), 13, true).unwrap();
    let new = parse_at_version(
        platform_value!({
            "type": "object",
            "properties": {
                "label": {"type": "integer", "position": 0},
            },
            "additionalProperties": false,
            "documentsKeepHistory": true,
            "canBeDeleted": false,
        }),
        14,
        true,
    )
    .unwrap();
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
        .unwrap();
    assert!(
        !result.is_valid(),
        "the withdrawal must not bypass schema compatibility"
    );
}

#[test]
fn should_reject_mutability_change_during_a_delete_flag_withdrawal() {
    let old = parse_at_version(repair_schema(true, true), 13, true).unwrap();
    let mut schema = repair_schema(true, false);
    schema.set_value("documentsMutable", false.into()).unwrap();
    let new = parse_at_version(schema, 14, true).unwrap();
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
        .unwrap();
    assert!(
        !result.is_valid(),
        "the withdrawal must not bypass other configuration checks"
    );
}

#[test]
fn should_still_validate_a_property_named_can_be_deleted_during_a_withdrawal() {
    let mut old_schema = repair_schema(true, true);
    old_schema
        .set_value(
            "properties",
            platform_value!({
                "canBeDeleted": {"type": "string", "maxLength": 50, "position": 0},
            }),
        )
        .unwrap();
    let mut new_schema = old_schema.clone();
    new_schema.set_value("canBeDeleted", false.into()).unwrap();
    new_schema
        .set_value(
            "properties",
            platform_value!({
                "canBeDeleted": {"type": "integer", "position": 0},
            }),
        )
        .unwrap();
    let old = parse_at_version(old_schema, 13, true).unwrap();
    let new = parse_at_version(new_schema, 14, true).unwrap();
    let result = old
        .as_ref()
        .validate_update(new.as_ref(), 2, PlatformVersion::get(14).unwrap())
        .unwrap();
    assert!(
        !result.is_valid(),
        "only the top-level config flag may be stripped"
    );
}
