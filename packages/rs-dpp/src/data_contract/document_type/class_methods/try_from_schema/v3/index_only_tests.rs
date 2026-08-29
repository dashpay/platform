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
fn accepts_summable_index() {
    // The sum axes are admitted on indexOnly indexes: the terminal entry
    // becomes an `ItemWithSumItem(commitment, amount)`. The summable
    // declaration goes through the same doctype-level aggregate
    // cross-checks as stored types (integer type, required membership),
    // and the summed property must still satisfy the indexOnly
    // every-property-indexed rule — here it joins byLiker's prefix.
    let mut schema = likes_schema_with_index_key(2, "summable", platform_value!("likeWeight"));
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .get_mut(2)
        .expect("index exists")
        .set_value(
            "properties",
            platform_value!([{ "$ownerId": "asc" }, { "likeWeight": "asc" }]),
        )
        .expect("index properties apply");
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
    let document_type =
        parse_with(schema, PlatformVersion::latest(), false).expect("summable index admitted");
    let summable_index = document_type
        .indices
        .values()
        .find(|index| index.summable.is_some())
        .expect("an index carries the summable declaration");
    assert_eq!(summable_index.summable.as_deref(), Some("likeWeight"));
}

#[test]
fn rejects_summable_naming_non_integer_property() {
    // The doctype-level aggregate cross-checks (shared with stored types)
    // still apply to indexOnly indexes: a summable naming a string
    // property fails the integer-type rule.
    let schema = likes_schema_with_index_key(2, "summable", platform_value!("hashtag"));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "integer type",
    );
}

#[test]
fn accepts_time_range_bucketed_index() {
    // A bucketed indexOnly index writes one entry per containing bucket,
    // sharing the stored types' walker fan-out. `$createdAt` must be the
    // transform source (the prefix rule admits no other system timestamp)
    // and must be required (shared timeRange rule).
    let mut schema = likes_schema();
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "$createdAt"]),
        )
        .expect("required applies");
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .push(platform_value!({
            "name": "byHourHashtag",
            "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
            "terminal": "$ownerId",
            "timeRange": { "on": "$createdAt", "range": 3600u64, "step": 900u64 },
            "countable": true,
            "rangeCountable": true
        }));
    let document_type = parse_with(schema, PlatformVersion::latest(), false)
        .expect("bucketed indexOnly index admitted");
    let bucketed = document_type
        .indices
        .values()
        .find(|index| index.time_range.is_some())
        .expect("the bucketed index parsed");
    assert_eq!(bucketed.time_range.as_ref().unwrap().overlap_factor(), 4);
}

#[test]
fn rejects_time_range_bucketed_index_without_required_created_at() {
    // Same shape, but $createdAt missing from `required` — the indexOnly
    // indexed-$createdAt rule fires (creation only assigns the timestamp
    // for required system times, and an entry cannot represent a missing
    // value).
    let mut schema = likes_schema();
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .push(platform_value!({
            "name": "byHourHashtag",
            "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
            "terminal": "$ownerId",
            "timeRange": { "on": "$createdAt", "range": 3600u64, "step": 900u64 }
        }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "must be listed in `required`",
    );
}

#[test]
fn rejects_only_bucketed_indexes() {
    // A bucketed index involves $createdAt, so a doctype whose every index
    // is bucketed has no $createdAt-free proof index and stays refused.
    let mut schema = likes_schema();
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "$createdAt"]),
        )
        .expect("required applies");
    schema
        .set_value(
            "indices",
            platform_value!([{
                "name": "byHourHashtagPost",
                "properties": [
                    { "$createdAt": "asc" },
                    { "hashtag": "asc" },
                    { "postId": "asc" }
                ],
                "terminal": "$ownerId",
                "timeRange": { "on": "$createdAt", "range": 3600u64, "step": 900u64 }
            }]),
        )
        .expect("indices apply");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "does not involve $createdAt",
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

/// The unified matcher: terminals participate as an index's deepest
/// matchable component, with generic (non-terminal) matches keeping
/// absolute precedence and difference-scored best-match inside each
/// class.
#[test]
fn terminal_aware_matching_prefers_generic_and_scores_candidates() {
    use crate::data_contract::document_type::methods::DocumentTypeV0Methods;

    let document_type_ref =
        &parse_with(likes_schema(), PlatformVersion::latest(), false).expect("likes schema parses");

    // A pure-property cover exists (`byPost` = [postId] → $ownerId used
    // generically): it must win over `byLiker`'s terminal cover of the
    // same field, and report the terminal unused.
    let (index, difference, terminal_used) = document_type_ref
        .index_for_types_matching_including_terminal(
            &["postId"],
            None,
            &[],
            |_| true,
            PlatformVersion::latest(),
        )
        .expect("matcher runs")
        .expect("an index matches");
    assert_eq!(index.name, "byPost");
    assert_eq!((difference, terminal_used), (0, false));

    // No pure-property cover for {$ownerId, postId}: `byLiker`
    // ([$ownerId] → postId) covers it exactly through its terminal
    // (difference 0) and must beat `byHashtagPost`'s costlier terminal
    // cover (hashtag unused, difference 1).
    let (index, difference, terminal_used) = document_type_ref
        .index_for_types_matching_including_terminal(
            &["$ownerId", "postId"],
            None,
            &[],
            |_| true,
            PlatformVersion::latest(),
        )
        .expect("matcher runs")
        .expect("an index matches");
    assert_eq!(index.name, "byLiker");
    assert_eq!((difference, terminal_used), (0, true));

    // The full tuple {hashtag, postId, $ownerId} is only coverable with
    // `byHashtagPost`'s terminal; an unused terminal never costs score,
    // so the exact cover reports difference 0.
    let (index, difference, terminal_used) = document_type_ref
        .index_for_types_matching_including_terminal(
            &["hashtag", "postId", "$ownerId"],
            None,
            &[],
            |_| true,
            PlatformVersion::latest(),
        )
        .expect("matcher runs")
        .expect("an index matches");
    assert_eq!(index.name, "byHashtagPost");
    assert_eq!((difference, terminal_used), (0, true));
}

#[test]
fn rejects_when_every_index_involves_created_at() {
    // Executed-transition proofs locate an entry from the transition's
    // values alone — a client verifier cannot reproduce the block
    // timestamp a time-keyed entry was written with. At least one index
    // must therefore stay $createdAt-free (the proof index); with every
    // index time-keyed, creates and deletes would work while every
    // wait-for-transition proof failed.
    let mut schema = likes_schema_with(
        "indices",
        platform_value!([
            {
                "name": "byHashtagPostTime",
                "properties": [
                    { "hashtag": "asc" },
                    { "postId": "asc" },
                    { "$createdAt": "asc" }
                ],
                "terminal": "$ownerId"
            }
        ]),
    );
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "$createdAt"]),
        )
        .expect("required applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "at least one index that does not involve $createdAt",
    );
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

// ── preallocated indexes ────────────────────────────────────────────────

/// Replaces `postId`'s `refersTo` declaration in place (`set_value` writes
/// literal top-level keys, so nested keys need the walk).
fn set_post_id_refers_to(schema: &mut Value, refers_to: Value) {
    schema
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .get_mut("postId")
        .expect("postId accessible")
        .expect("postId present")
        .set_value("refersTo", refers_to)
        .expect("refersTo applies");
}

/// The likes schema with `postId` carrying a same-contract
/// permanentDocument reference to `post` agreeing on `hashtag` — the shape
/// whose `byHashtagPost` and `byPost` paths are pure functions of the
/// referenced post, so both may declare `preallocated`.
fn preallocatable_likes_schema() -> Value {
    let mut schema = likes_schema();
    set_post_id_refers_to(
        &mut schema,
        platform_value!({
            "type": "permanentDocument",
            "documentType": "post",
            "propertyAgreement": { "hashtag": "hashtag" }
        }),
    );
    schema
}

#[test]
fn preallocated_accepts_a_reference_determined_index() {
    for full_validation in [false, true] {
        let mut schema = preallocatable_likes_schema();
        for index_position in [0, 1] {
            schema
                .get_mut("indices")
                .expect("indices accessible")
                .expect("indices present")
                .as_array_mut()
                .expect("indices is an array")
                .get_mut(index_position)
                .expect("index exists")
                .set_value("preallocated", Value::Bool(true))
                .expect("index key applies");
        }
        let document_type = parse_with(schema, PlatformVersion::latest(), full_validation)
            .expect("a reference-determined preallocated index parses");
        assert!(
            document_type
                .indices
                .get("byHashtagPost")
                .unwrap()
                .preallocated
        );
        assert!(document_type.indices.get("byPost").unwrap().preallocated);
        assert!(!document_type.indices.get("byLiker").unwrap().preallocated);

        // The flag is stamped onto the terminating index level, where the
        // rs-drive delete walker reads it to skip upward pruning.
        let hashtag_level = document_type
            .index_structure
            .sub_levels()
            .get("hashtag")
            .unwrap();
        let post_level = hashtag_level.sub_levels().get("postId").unwrap();
        assert!(post_level.has_index_with_type().unwrap().preallocated);
        let owner_level = document_type
            .index_structure
            .sub_levels()
            .get("$ownerId")
            .unwrap();
        assert!(!owner_level.has_index_with_type().unwrap().preallocated);
    }
}

#[test]
fn rejects_preallocated_without_a_document_reference() {
    // The base schema's `postId` refers to an identity — no referenced
    // document determines the path.
    expect_structure_error(
        parse_with(
            likes_schema_with_index_key(0, "preallocated", Value::Bool(true)),
            PlatformVersion::latest(),
            false,
        ),
        "not determined by a reference",
    );
}

#[test]
fn rejects_preallocated_with_an_uncovered_property() {
    // A permanentDocument reference WITHOUT the hashtag agreement leaves
    // byHashtagPost's `hashtag` undetermined.
    let mut schema = likes_schema_with_index_key(0, "preallocated", Value::Bool(true));
    set_post_id_refers_to(
        &mut schema,
        platform_value!({
            "type": "permanentDocument",
            "documentType": "post"
        }),
    );
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "not determined by a reference",
    );

    // `byPost` ([postId], terminal $ownerId) stays fully determined by the
    // reference alone, so the same schema with the flag there parses.
    let mut schema = likes_schema_with_index_key(1, "preallocated", Value::Bool(true));
    set_post_id_refers_to(
        &mut schema,
        platform_value!({
            "type": "permanentDocument",
            "documentType": "post"
        }),
    );
    parse_with(schema, PlatformVersion::latest(), false)
        .expect("an agreement-free reference still determines a [postId] index");
}

/// The preallocatable schema with `preallocated: true` on the index at
/// `index_position` (0 = byHashtagPost, 1 = byPost, 2 = byLiker).
fn preallocatable_likes_schema_with_flag_on(index_position: usize) -> Value {
    let mut schema = preallocatable_likes_schema();
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .get_mut(index_position)
        .expect("index exists")
        .set_value("preallocated", Value::Bool(true))
        .expect("index key applies");
    schema
}

#[test]
fn rejects_preallocated_on_owner_prefixed_index() {
    // byLiker's `$ownerId` prefix can never be determined by the
    // referenced document.
    expect_structure_error(
        parse_with(
            preallocatable_likes_schema_with_flag_on(2),
            PlatformVersion::latest(),
            false,
        ),
        "not determined by a reference",
    );
}

#[test]
fn rejects_preallocated_on_cross_contract_reference() {
    // The reference names a DIFFERENT contract — its document inserts
    // happen in a subtree this contract's insert path never touches.
    let mut schema = likes_schema_with_index_key(1, "preallocated", Value::Bool(true));
    set_post_id_refers_to(
        &mut schema,
        platform_value!({
            "type": "permanentDocument",
            "contractId": Value::Identifier([2; 32]),
            "documentType": "post"
        }),
    );
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "not determined by a reference",
    );

    // Naming the declaring contract's own id explicitly counts as
    // same-contract (the parse helper registers under [1; 32]).
    let mut schema = likes_schema_with_index_key(1, "preallocated", Value::Bool(true));
    set_post_id_refers_to(
        &mut schema,
        platform_value!({
            "type": "permanentDocument",
            "contractId": Value::Identifier([1; 32]),
            "documentType": "post"
        }),
    );
    parse_with(schema, PlatformVersion::latest(), false)
        .expect("an own-id reference is a same-contract reference");
}

#[test]
fn rejects_preallocated_on_non_index_only_type() {
    let mut schema = preallocatable_likes_schema();
    schema
        .remove_optional_value("indexOnly")
        .expect("removal applies");
    for index_value in schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
    {
        let _ = index_value.remove_optional_value("terminal");
    }
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .get_mut(1)
        .expect("byPost exists")
        .set_value("preallocated", Value::Bool(true))
        .expect("index key applies");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "declares `preallocated`",
    );
}

#[test]
fn preallocated_keyword_is_rejected_below_generation_3_on_both_modes() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");
    let schema = preallocatable_likes_schema_with_flag_on(1);

    for full_validation in [false, true] {
        assert!(
            parse_dispatched(schema.clone(), platform_version_13, full_validation).is_err(),
            "PV13 must reject the preallocated keyword (full_validation: {full_validation}): \
             it is not part of generation 2's index grammar"
        );
    }
}

#[test]
fn rejects_preallocated_on_bucketed_index() {
    // A bucketed level is keyed by grid-qualified bucket starts computed
    // from a timestamp at write time — nothing a referenced document could
    // determine, so `preallocated` + `timeRange` is rejected outright.
    let mut schema = preallocatable_likes_schema();
    schema
        .set_value(
            "required",
            platform_value!(["hashtag", "postId", "$createdAt"]),
        )
        .expect("required applies");
    schema
        .get_mut("indices")
        .expect("indices accessible")
        .expect("indices present")
        .as_array_mut()
        .expect("indices is an array")
        .push(platform_value!({
            "name": "byHourHashtag",
            "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
            "terminal": "$ownerId",
            "timeRange": { "on": "$createdAt", "range": 3600u64, "step": 900u64 },
            "countable": true,
            "rangeCountable": true,
            "preallocated": true
        }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "declares `preallocated` together with `timeRange`",
    );
}
