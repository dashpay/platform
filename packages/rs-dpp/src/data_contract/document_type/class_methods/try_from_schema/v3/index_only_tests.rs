//! indexOnly document types — parser-generation gating and the structural
//! constraint matrix.
//!
//! The `indexOnly` doctype keyword and the `terminal` index keyword joined
//! the grammar at generation 3 (meta-schema v3, protocol version 14), like
//! the ranked keywords next door. The same two-sided gating has to hold on
//! the pre-PV14 side:
//!
//!   * `terminal` (index-level, strict grammar): rejected as an unknown key
//!     on BOTH validation modes below generation 3.
//!   * `indexOnly` (doctype-level, loose grammar): ignored by earlier
//!     generations on the non-validating path — exactly how every
//!     doctype-level keyword they predate behaves — and rejected by their
//!     meta-schemas under `full_validation`.
//!
//! The constraint matrix itself (`apply_index_only`) runs regardless of
//! `full_validation` because the on-disk layout depends on it, so every
//! rejection test here goes through the non-validating parse — the
//! smuggling path — and the happy path is additionally exercised under full
//! validation to pin the meta-schema admission.

use super::*;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::errors::DataContractError;
use platform_value::platform_value;

/// Parse through this generation with validation mode spelled out.
fn parse_with(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentTypeV2, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    try_from_schema_generation_3(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "like",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// Parse through the real dispatcher, which picks the parser generation out
/// of the platform version's `try_from_schema` table value (generation 2 at
/// PV13, generation 3 at PV14).
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
        "like",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// The worked example from the design: a Yappr-style `like` type. Two
/// schema properties (`hashtag`, `postId` — the latter a refersTo-typed
/// identifier), three indexes:
///
///   * `byHashtagPost` — ranked per-hashtag top posts, terminal `$ownerId`
///   * `byPost` — global ranking + one-like-per-(post, owner) constraint;
///     terminal omitted to exercise the `$ownerId` default
///   * `byLiker` — "what did I like", `$ownerId` prefix, `postId` terminal
fn likes_schema() -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": {
            "hashtag": {
                "type": "string",
                "maxLength": 63,
                "position": 0
            },
            "postId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "refersTo": { "type": "identity" },
                "position": 1
            }
        },
        "required": ["hashtag", "postId"],
        "indices": [
            {
                "name": "byHashtagPost",
                "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId",
                "countable": true,
                "rangeCountable": true,
                "rankedCountable": true
            },
            {
                "name": "byPost",
                "properties": [{ "postId": "asc" }],
                "countable": true,
                "rangeCountable": true,
                "rankedCountable": true
            },
            {
                "name": "byLiker",
                "properties": [{ "$ownerId": "asc" }],
                "terminal": "postId"
            }
        ],
        "additionalProperties": false
    })
}

/// The base schema with one doctype-level key set (added or replaced).
fn likes_schema_with(key: &str, value: Value) -> Value {
    let mut schema = likes_schema();
    schema.set_value(key, value).expect("doctype key applies");
    schema
}

/// The base schema with one key set (added or replaced) on the index at
/// `index_position` (0 = byHashtagPost, 1 = byPost, 2 = byLiker).
fn likes_schema_with_index_key(index_position: usize, key: &str, value: Value) -> Value {
    let mut schema = likes_schema();
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .get_mut(index_position)
        .expect("index exists")
        .set_value(key, value)
        .expect("index key applies");
    schema
}

fn expect_structure_error(result: Result<DocumentTypeV2, ProtocolError>, needle: &str) {
    match result {
        Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            message,
        ))) => {
            assert!(
                message.contains(needle),
                "expected structure error containing {needle:?}, got: {message}"
            );
        }
        Err(other) => {
            panic!("expected InvalidContractStructure containing {needle:?}, got {other}")
        }
        Ok(_) => panic!("expected rejection containing {needle:?}, but the schema parsed"),
    }
}

// ── the happy path ──────────────────────────────────────────────────────

#[test]
fn likes_schema_parses_on_both_validation_modes() {
    let platform_version = PlatformVersion::latest();

    for full_validation in [false, true] {
        let document_type = parse_with(likes_schema(), platform_version, full_validation)
            .unwrap_or_else(|error| {
                panic!("likes schema should parse (full_validation: {full_validation}): {error}")
            });

        assert!(document_type.index_only());
        assert!(!document_type.documents_mutable);
        assert_eq!(document_type.indices.len(), 3);
    }
}

#[test]
fn terminal_defaults_to_owner_id() {
    let document_type =
        parse_with(likes_schema(), PlatformVersion::latest(), false).expect("should parse");

    // `byPost` omits its terminal; normalization spells it out, so the
    // omitted and explicit forms parse to equal indexes.
    assert_eq!(
        document_type.indices["byPost"].terminal.as_deref(),
        Some("$ownerId")
    );
    assert_eq!(
        document_type.indices["byHashtagPost"].terminal.as_deref(),
        Some("$ownerId")
    );
    assert_eq!(
        document_type.indices["byLiker"].terminal.as_deref(),
        Some("postId")
    );
}

// ── doctype-level constraint matrix (non-validating parse: the checks are
//    structural, not meta-schema lints) ─────────────────────────────────

#[test]
fn rejects_mutable_documents() {
    let schema = likes_schema_with("documentsMutable", Value::Bool(true));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "documentsMutable: false",
    );
}

#[test]
fn rejects_transferable_documents() {
    let schema = likes_schema_with("transferable", Value::U8(1));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "must not be transferable",
    );
}

#[test]
fn rejects_keep_history() {
    let schema = likes_schema_with("documentsKeepHistory", Value::Bool(true));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "cannot keep history",
    );
}

#[test]
fn rejects_transient_properties() {
    let schema = likes_schema_with("transient", platform_value!(["hashtag"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "transient",
    );
}

#[test]
fn rejects_doctype_level_aggregates() {
    let schema = likes_schema_with("documentsCountable", Value::Bool(true));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "doctype-level aggregate keywords",
    );
}

#[test]
fn rejects_missing_indices() {
    let schema = likes_schema_with("indices", platform_value!([]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "at least one index",
    );
}

// ── per-index constraint matrix ─────────────────────────────────────────

#[test]
fn rejects_unique_indexes() {
    let schema = likes_schema_with_index_key(2, "unique", Value::Bool(true));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "cannot be unique",
    );
}

#[test]
fn rejects_null_searchable_false() {
    let schema = likes_schema_with_index_key(2, "nullSearchable", Value::Bool(false));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "nullSearchable",
    );
}

#[test]
fn rejects_sum_axes() {
    // The summable declaration itself has to survive the aggregate
    // cross-checks (integer type, required) so that the indexOnly-specific
    // rejection is the one that fires.
    let mut schema = likes_schema_with_index_key(2, "summable", platform_value!("likeWeight"));
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "likeWeight",
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 100, "position": 2 }),
        )
        .expect("property applies");
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "likeWeight"]),
        )
        .expect("required applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "sum axes",
    );
}

#[test]
fn rejects_terminal_repeating_an_index_property() {
    // byLiker's prefix is [$ownerId]; making $ownerId its terminal too
    // indexes the same dimension twice.
    let schema = likes_schema_with_index_key(2, "terminal", platform_value!("$ownerId"));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "repeats its terminal",
    );
}

#[test]
fn rejects_terminal_without_refers_to() {
    // `hashtag` is a plain string — not `$ownerId`, not a refersTo-typed
    // identifier — so it cannot be a member key.
    let schema = likes_schema_with_index_key(2, "terminal", platform_value!("hashtag"));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "refersTo",
    );
}

#[test]
fn rejects_terminal_naming_unknown_property() {
    let schema = likes_schema_with_index_key(2, "terminal", platform_value!("nonsense"));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "does not name a property",
    );
}

#[test]
fn rejects_disallowed_system_properties_in_prefix() {
    let schema =
        likes_schema_with_index_key(2, "properties", platform_value!([{ "$updatedAt": "asc" }]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "$ownerId and $createdAt",
    );
}

#[test]
fn accepts_created_at_in_prefix_when_required() {
    // `$createdAt` is assigned from block time at create (only when it is
    // required — which indexing it therefore demands) and recoverable from
    // the entry path — the one system property besides `$ownerId` an
    // indexOnly index may carry.
    let mut schema = likes_schema_with_index_key(
        2,
        "properties",
        platform_value!([{ "$ownerId": "asc" }, { "$createdAt": "asc" }]),
    );
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "$createdAt"]),
        )
        .expect("required applies");
    parse_with(schema, PlatformVersion::latest(), false)
        .expect("$createdAt in an index prefix should be accepted when required");
}

#[test]
fn rejects_indexed_created_at_that_is_not_required() {
    // Document creation assigns `created_at` only for a REQUIRED
    // $createdAt; indexing an unrequired one would silently take the
    // missing-value branch instead of storing block time.
    let schema = likes_schema_with_index_key(
        2,
        "properties",
        platform_value!([{ "$ownerId": "asc" }, { "$createdAt": "asc" }]),
    );
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "\"$createdAt\" must be listed in `required`",
    );
}

#[test]
fn rejects_identity_public_key_reference_terminals() {
    // identityPublicKey is a compound reference (identity id here, key id
    // in a companion property) — the member key alone cannot identify the
    // referenced key, so it is not a legal terminal.
    let mut schema = likes_schema_with_index_key(2, "terminal", platform_value!("keyRef"));
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "keyRef",
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "refersTo": { "type": "identityPublicKey", "keyIdProperty": "keyId" },
                "position": 2
            }),
        )
        .expect("property applies");
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "keyId",
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 100, "position": 3 }),
        )
        .expect("property applies");
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "keyRef", "keyId"]),
        )
        .expect("required applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "identityPublicKey",
    );
}

#[test]
fn rejects_owner_less_indexes() {
    // Every index must be bound to its owner: an owner-less index would
    // let a crafted delete splice a victim's row in with the signer's own
    // owner-bearing row.
    let schema =
        likes_schema_with_index_key(2, "properties", platform_value!([{ "hashtag": "asc" }]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "must include $ownerId",
    );
}

// ── coverage, requiredness, ownership ───────────────────────────────────

#[test]
fn rejects_unindexed_property() {
    // Drop the compound index entirely: `hashtag` then appears in no
    // index and would be silently unrecoverable.
    let schema = likes_schema_with(
        "indices",
        platform_value!([
            {
                "name": "byPost",
                "properties": [{ "postId": "asc" }],
                "countable": true,
                "rangeCountable": true,
                "rankedCountable": true
            },
            {
                "name": "byLiker",
                "properties": [{ "$ownerId": "asc" }],
                "terminal": "postId"
            }
        ]),
    );
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "does not appear in any index",
    );
}

#[test]
fn rejects_optional_property() {
    let schema = likes_schema_with("required", platform_value!(["postId"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "must be listed in `required`",
    );
}

#[test]
fn rejects_missing_owner_id() {
    // An index carrying no $ownerId anywhere is refused outright — every
    // entry must be self-authorizing for deletes.
    let schema = likes_schema_with(
        "indices",
        platform_value!([
            {
                "name": "byHashtag",
                "properties": [{ "hashtag": "asc" }],
                "terminal": "postId"
            }
        ]),
    );
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "must include $ownerId",
    );
}

// ── terminal without indexOnly ──────────────────────────────────────────

#[test]
fn rejects_terminal_on_non_index_only_type() {
    let mut schema = likes_schema();
    schema
        .remove_optional_value("indexOnly")
        .expect("removal applies");
    // Without indexOnly the doctype-flag constraints don't apply, but the
    // dangling `terminal` declarations must be rejected.
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "declares `terminal`",
    );
}

// ── cross-generation gating ─────────────────────────────────────────────

#[test]
fn terminal_keyword_is_rejected_below_generation_3_on_both_modes() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");

    for full_validation in [false, true] {
        let result = parse_dispatched(likes_schema(), platform_version_13, full_validation);
        assert!(
            result.is_err(),
            "PV13 must reject the likes schema (full_validation: {full_validation}): \
             `terminal` is not part of generation 2's index grammar"
        );
    }
}

#[test]
fn index_only_keyword_is_inert_below_generation_3_without_validation() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");

    // A doctype-level `indexOnly` with no `terminal` keywords: generation 2
    // ignores unknown doctype keys on the non-validating path — exactly how
    // it treats every keyword it predates — so the parse succeeds and the
    // flag is NOT set.
    let mut schema = likes_schema();
    for index_value in schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
    {
        // Strip the generation-3 index grammar so only `indexOnly` remains.
        let _ = index_value.remove_optional_value("terminal");
        let _ = index_value.remove_optional_value("rankedCountable");
    }

    let document_type = parse_dispatched(schema.clone(), platform_version_13, false)
        .expect("generation 2 ignores unknown doctype-level keywords when not validating");
    assert!(
        !document_type.index_only(),
        "generation 2 must not set index_only"
    );

    // Under full validation the v2 meta-schema rejects the unknown key.
    assert!(
        parse_dispatched(schema, platform_version_13, true).is_err(),
        "meta-schema v2 must reject the indexOnly keyword"
    );
}

#[test]
fn index_only_flag_survives_the_dispatcher_at_latest() {
    let document_type = parse_dispatched(likes_schema(), PlatformVersion::latest(), true)
        .expect("likes schema parses through the dispatcher at PV14");
    assert!(document_type.index_only());
}
