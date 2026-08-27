//! Document-type parser **generation 3** — protocol version 14 and later.
//!
//! Generation 3 is generation 2 plus the ranked index keywords
//! (`rankedCountable` / `rankedSummable` / `rankedAverageable`).
//!
//! It exists as its own generation — rather than as a version gate inside the
//! shipped ones — because that is what keeps a historical block from ever
//! picking up grammar that did not exist when it was produced: the dispatcher
//! only routes here from `try_from_schema: 3`, and the grammar this module
//! admits is spelled out below as literals rather than looked up in a version
//! table.
//!
//! The parsing steps themselves are shared with the earlier generations in
//! [`super::common`]. What stays here is what only generation 3 has: the ranked
//! index-key length ceilings, and the constants they are derived from.

use crate::data_contract::config::DataContractConfig;
// Only the ranked key-length rule below names `Index`, and it is validation-only.
#[cfg(feature = "validation")]
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index::IndexGrammarAdmissions;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::InvalidIndexedPropertyConstraintError;

use super::common;

mod ranked_prefix_overlap;
use ranked_prefix_overlap::validate_no_ranked_prefix_overlap;

/// grovedb's ceiling on the key of an entry stored directly under an
/// *indexed* tree's primary when the tree carries only the Count and/or Sum
/// axes (`grovedb::operations::indexed_tree::MAX_CIDX_ITEM_KEY_LEN`).
///
/// Those two axes key their ordered secondary by `sort_key ‖ item_key` with
/// an 8-byte sort key, and Merk requires every key to stay below 256 bytes,
/// which leaves `255 - 8 = 247` bytes for the item key. GroveDB enforces it
/// on both write paths (the dedicated insert and the batch pre-state check),
/// so a document whose index key exceeded it would abort the whole batch.
#[cfg(feature = "validation")]
const MAX_RANKED_COUNT_SUM_INDEX_KEY_LENGTH: u16 = 247;

/// The same ceiling for a ranked tree whose configured axes include Avg
/// (`grovedb::operations::indexed_tree::MAX_AVG_INDEXED_ITEM_KEY_LEN`).
///
/// The avg secondary sorts on a 16-byte fixed-point key rather than an
/// 8-byte one, so the item key gets 8 fewer bytes: `255 - 16 = 239`.
#[cfg(feature = "validation")]
const MAX_RANKED_AVG_INDEX_KEY_LENGTH: u16 = 239;

/// Worst-case bytes one character of a string property can occupy once it has
/// been encoded into an index key.
///
/// `DocumentPropertyType::encode_value_for_tree_keys` stores a string as its
/// raw UTF-8 bytes, and a single UTF-8 scalar takes at most 4 bytes — which
/// is exactly the factor `DocumentPropertyType::max_byte_size` applies to
/// `maxLength`, and the factor the generic
/// [`MAX_INDEXED_STRING_PROPERTY_LENGTH`] was derived from
/// (`floor(255 / 4) = 63`).
#[cfg(feature = "validation")]
const INDEXED_STRING_WORST_CASE_BYTES_PER_CHARACTER: u16 = 4;

/// The strictest item-key ceiling the index's declared ranking axes impose,
/// or `None` when the index declares no ranking axis at all (in which case
/// only the generic index-key limits apply).
///
/// Avg wins when present because its wider sort key leaves the least room;
/// the three flags are independent, so an index may carry Avg alongside
/// Count and/or Sum and still has to satisfy the tightest of them.
#[cfg(feature = "validation")]
fn ranked_index_key_length_limit(index: &Index) -> Option<u16> {
    if index.ranked_averageable {
        Some(MAX_RANKED_AVG_INDEX_KEY_LENGTH)
    } else if index.ranked_countable || index.ranked_summable {
        Some(MAX_RANKED_COUNT_SUM_INDEX_KEY_LENGTH)
    } else {
        None
    }
}

/// Reject an index property whose *worst-case* encoded key would not fit
/// under the ceiling its index's ranking axes impose.
///
/// A ranked index turns the property-name tree into a grovedb indexed tree
/// whose children — the per-group value trees, keyed by the encoded property
/// value — are mirrored into an ordered secondary keyed by
/// `sort_key ‖ item_key`. The generic index-key limits (255 encoded bytes for
/// a byte array, 63 characters for a string) were derived against Merk's bare
/// 256-byte key ceiling and are therefore too loose here by the width of the
/// sort key. Contracts must fail at registration rather than let a document
/// insert abort the block's batch on a key grovedb refuses to mirror.
///
/// The worst case is derived exactly the way every other size-sensitive path
/// derives it, through [`DocumentPropertyType::max_byte_size`]: a string of
/// `maxLength` L characters can reach `4 * L` bytes, a byte array of
/// `maxItems` N reaches N, and every other indexable type has a fixed
/// encoding of at most 32 bytes (identifiers) that can never reach a ceiling
/// in the 239..=247 range.
#[cfg(feature = "validation")]
fn validate_ranked_index_property_key_length(
    document_type_name: &str,
    index: &Index,
    index_property_name: &str,
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    // Only the **terminal** property's encoded value becomes an
    // indexed-tree item key, mirrored into the ordered secondary behind
    // the sort key — that is where the tightened ceiling comes from.
    // Leading properties of a compound ranked index are ordinary grovedb
    // path segments, bound by the generic limits checked after this.
    if index.properties.last().map(|p| p.name.as_str()) != Some(index_property_name) {
        return Ok(());
    }

    let Some(limit) = ranked_index_key_length_limit(index) else {
        return Ok(());
    };

    // `None` is only produced by the array and object types, which the
    // property-type check right after this one rejects outright with the
    // error that actually explains the problem.
    let Some(worst_case_key_length) = property_type.max_byte_size(platform_version)? else {
        return Ok(());
    };

    if worst_case_key_length <= limit {
        return Ok(());
    }

    // The sort-key width is what the ceiling gives up against Merk's
    // 255-byte maximum key, so it reads straight back out of the limit.
    let sort_key_width = 255 - limit;

    let (constraint_name, reason) = match property_type {
        DocumentPropertyType::String(_) => (
            "maxLength",
            format!(
                "should be less or equal {} on an index declaring a ranked axis: the index key \
                 is the property's UTF-8 bytes (worst case {} bytes per character), and grovedb \
                 caps a ranked group key at {} bytes because the ordered secondary is keyed by a \
                 {}-byte sort key followed by the group key and Merk keys must stay below 256 \
                 bytes",
                limit / INDEXED_STRING_WORST_CASE_BYTES_PER_CHARACTER,
                INDEXED_STRING_WORST_CASE_BYTES_PER_CHARACTER,
                limit,
                sort_key_width,
            ),
        ),
        DocumentPropertyType::ByteArray(_) => (
            "maxItems",
            format!(
                "should be less or equal {limit} on an index declaring a ranked axis: grovedb \
                 caps a ranked group key at {limit} bytes because the ordered secondary is keyed \
                 by a {sort_key_width}-byte sort key followed by the group key and Merk keys must \
                 stay below 256 bytes",
            ),
        ),
        // Unreachable for every type the parser admits here — they all
        // encode to at most 32 bytes — but stated rather than assumed so a
        // future variable-width type cannot slip past unbounded.
        _ => (
            "maximum encoded size",
            format!(
                "the property's worst-case encoded index key is {worst_case_key_length} bytes, \
                 but an index declaring a ranked axis caps its group key at {limit} bytes (the \
                 ordered secondary is keyed by a {sort_key_width}-byte sort key followed by the \
                 group key and Merk keys must stay below 256 bytes)",
            ),
        ),
    };

    Err(ProtocolError::ConsensusError(Box::new(
        InvalidIndexedPropertyConstraintError::new(
            document_type_name.to_owned(),
            index.name.to_owned(),
            index_property_name.to_owned(),
            constraint_name.to_string(),
            reason,
        )
        .into(),
    )))
}

/// The [`common::RankedIndexKeyLengthCheck`] generation 3 runs on every indexed
/// property, resolved at compile time.
///
/// Without the `validation` feature the shared core never reaches the check at
/// all (its whole `full_validation` block is compiled out), so the no-op is
/// exactly equivalent there — this alias exists only so the call site can pass
/// the check unconditionally.
#[cfg(feature = "validation")]
const RANKED_INDEX_KEY_LENGTH_CHECK: common::RankedIndexKeyLengthCheck =
    validate_ranked_index_property_key_length;
#[cfg(not(feature = "validation"))]
const RANKED_INDEX_KEY_LENGTH_CHECK: common::RankedIndexKeyLengthCheck =
    common::no_ranked_index_key_length_check;

/// Parses a document type schema through the generation-3 grammar: the
/// generation-2 doctype-level aggregate keywords, plus the ranked index
/// keywords and the tighter index-key ceilings they impose.
///
/// This parser is only reachable from protocol version 14+ (via
/// CONTRACT_VERSIONS_V6).
#[allow(clippy::too_many_arguments)]
fn try_from_schema_generation_3(
    data_contract_id: Identifier,
    data_contract_system_version: u16,
    contract_config_version: u16,
    name: &str,
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
    data_contact_config: &DataContractConfig,
    full_validation: bool,
    validation_operations: &mut impl Extend<ProtocolValidationOperation>,
    platform_version: &PlatformVersion,
) -> Result<DocumentTypeV2, ProtocolError> {
    // Read the aggregate and indexOnly keywords before the core parser
    // consumes `schema`.
    let aggregates = common::parse_doctype_aggregate_keywords(&schema, name)?;
    let index_only = common::parse_index_only_keyword(&schema)?;

    let v1 = common::parse_document_type_core(
        data_contract_id,
        data_contract_system_version,
        contract_config_version,
        name,
        schema,
        schema_defs,
        token_configurations,
        data_contact_config,
        full_validation,
        // Lets the core default omitted index terminals to `$ownerId` before
        // it builds the index structure, so the structure's level info is
        // born normalized (`apply_index_only` below validates the
        // already-normalized set).
        index_only,
        validation_operations,
        &common::ParserGeneration {
            // Generation 3 exists if and only if `document_type_schema` is 3 —
            // CONTRACT_VERSIONS_V6 is the only table that selects this parser,
            // and it is the only table naming meta-schema v3. So every constant
            // here is a property of the generation, not of a platform version,
            // and none of them is read out of a table.
            document_type_schema_version: 3,
            // Meta-schema v3 carries the `keeps*History` flags forward from v2,
            // so they are unconditionally part of this generation's grammar.
            // This is the conditional that was dead in the copied core: at
            // schema 3 the old `document_type_schema >= 2` could only be true.
            admit_history: true,
            // Count indexes arrived at PV12; every version selecting this
            // generation is far past that boundary.
            admit_count_indexes: true,
            meta_schema_method_name: "DocumentType::try_from_schema_v3 (document_type_schema)",
            // RANKED / TIME RANGE: the keyword admissions that make this
            // generation 3, read from the shared generation → admission
            // mapping so the registration-cost re-parse can never drift
            // from what this parser accepts.
            admit_ranked: IndexGrammarAdmissions::for_schema_generation(3).ranked,
            ranked_index_key_length_check: RANKED_INDEX_KEY_LENGTH_CHECK,
            ranked_index_structure_check: validate_no_ranked_prefix_overlap,
            admit_time_range: IndexGrammarAdmissions::for_schema_generation(3).time_range,
            // INDEX ONLY: the `terminal` index keyword, admitted from the
            // same shared generation → admission mapping as the two above.
            admit_index_terminal: IndexGrammarAdmissions::for_schema_generation(3).terminal,
        },
        platform_version,
    )?;

    let mut v2: DocumentTypeV2 = v1.into();
    common::apply_doctype_aggregates(&mut v2, aggregates, name)?;
    // After the aggregates: `apply_index_only` rejects the doctype-level
    // aggregate flags (they describe the primary-key tree, which an
    // indexOnly type does not have), so it has to see them already applied.
    common::apply_index_only(&mut v2, index_only, name)?;

    Ok(v2)
}

impl DocumentType {
    /// Dispatches to this module's generation-3 parser and wraps the result.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract::document_type::class_methods) fn try_from_schema_v3(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        try_from_schema_generation_3(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )
        .map(DocumentType::V2)
    }
}

#[cfg(test)]
mod index_only_tests;

#[cfg(test)]
mod tests {
    //! Ranked aggregate index keywords — parser-generation gating.
    //!
    //! The keywords live on the *index*, not the document type, but what
    //! admits them is the parser generation the dispatcher selects
    //! (`try_from_schema`: 2 at PV13, 3 at PV14), so the tests belong on this
    //! parser. Two independent halves have to hold on the pre-PV14 side:
    //!
    //!   * `full_validation: true`  — the v2 meta-schema has
    //!     `additionalProperties: false` on index entries and rejects the key.
    //!   * `full_validation: false` — no meta-schema runs at all (check_tx,
    //!     cache warm-up, restore), so the *grammar* itself must not know the
    //!     keyword. That is the smuggling path, and the one worth pinning.
    //!
    //! The PV13 half of every test goes through the real dispatcher
    //! (`DocumentType::try_from_schema`) rather than through anything in this
    //! module, so what it exercises is generation 2 refusing the keys — the
    //! cross-generation behavior, not this generation's internals.
    use super::*;
    use platform_value::platform_value;

    /// Parse through **this** generation, with the platform version and
    /// validation mode spelled out. Used for the PV14 side, which has to be
    /// exercised on both the meta-schema path (`full_validation: true`) and the
    /// structural path (`full_validation: false`).
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

    /// Parse through the **real dispatcher**, which picks the parser generation
    /// out of the platform version's `try_from_schema` table value. At PV13
    /// that is generation 2; at PV14, this module.
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

    /// A `review` doctype with one index over `restaurantId`, averageable on
    /// `grade`, optionally carrying ranked keywords. Written so the v3
    /// meta-schema's prerequisite rules are satisfied: the ranked
    /// `if`/`then` conditionals demand the literal range keys, and the
    /// `dependentRequired` chain covers the rest
    /// (`rangeAverageable` → `averageable`).
    ///
    /// `restaurantId` is capped at 32 characters — comfortably inside the
    /// ranked key bound on every axis, so these grammar tests exercise the
    /// keywords and nothing else. The bound itself is exercised by
    /// [`ranked_review_schema_with_max_length`] below.
    fn ranked_review_schema(ranked_keys: Vec<(&str, bool)>) -> Value {
        ranked_review_schema_with_max_length(32, ranked_keys)
    }

    /// [`ranked_review_schema`] with the indexed string property's
    /// `maxLength` spelled out, for the ranked key-length boundary tests.
    fn ranked_review_schema_with_max_length(
        max_length: u32,
        ranked_keys: Vec<(&str, bool)>,
    ) -> Value {
        let mut index_entry: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("restaurantId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("grade".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
        ];
        for (key, value) in ranked_keys {
            index_entry.push((Value::Text(key.to_string()), Value::Bool(value)));
        }

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "restaurantId": {
                        "type": "string",
                        "maxLength": max_length,
                        "position": 0,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 1,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![
                    Value::Text("restaurantId".to_string()),
                    Value::Text("grade".to_string()),
                ]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![Value::Map(index_entry)]),
            ),
        ])
    }

    fn pv13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("protocol version 13 exists")
    }

    /// Generation-specific tests must pin a protocol version that actually
    /// selects their own generation: `pv14()` silently
    /// retargets these tests onto a different parser generation and a
    /// different document meta-schema whenever LATEST moves. PV14 is the
    /// first protocol version whose `try_from_schema` selects generation 3.
    fn pv14() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("protocol version 14 exists")
    }

    /// PV14 accepts the ranked keywords and carries them onto the parsed index
    /// — both when parsed through this generation directly and when reached
    /// the way production reaches it, through the dispatcher. The dispatcher
    /// half is what pins that `try_from_schema: 3` actually routes here.
    #[test]
    fn ranked_keywords_accepted_at_pv14() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema.clone(), pv14(), true)
            .expect("meta-schema v3 must accept the ranked index keywords");

        let index = v2
            .indices
            .get("byRestaurant")
            .expect("index parsed under its name");
        assert!(index.ranked_averageable);
        assert!(!index.ranked_countable);
        assert!(!index.ranked_summable);
        assert!(index.range_countable && index.range_summable);

        // Same schema, same platform version, through the real dispatcher.
        let dispatched = parse_dispatched(schema, pv14(), true)
            .expect("the dispatcher must route PV14 to a generation that accepts the keywords");
        let DocumentType::V2(dispatched) = dispatched else {
            panic!("generation 3 produces a V2-shaped document type");
        };
        assert!(
            dispatched
                .indices
                .get("byRestaurant")
                .expect("index parsed under its name")
                .ranked_averageable,
            "dispatching at PV14 must reach generation 3, not an earlier generation"
        );
    }

    /// PV13 + `full_validation`: generation 2's meta-schema rejects the unknown
    /// index key outright. Routed through the dispatcher so it is genuinely
    /// generation 2 doing the rejecting.
    #[test]
    fn ranked_keywords_rejected_at_pv13_under_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let result = parse_dispatched(schema, pv13(), true);
        assert!(
            result.is_err(),
            "meta-schema v2 must reject rankedAverageable (additionalProperties: false)"
        );
    }

    /// PV13 without `full_validation`: no meta-schema runs, so the structural
    /// grammar has to do the rejecting. Both `true` and `false` are rejected —
    /// the key's mere presence is what a pre-PV14 node refuses, and matching
    /// that exactly is what keeps replay of historical blocks identical.
    #[test]
    fn ranked_keywords_rejected_at_pv13_without_full_validation() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            for value in [true, false] {
                let schema = ranked_review_schema(vec![(key, value)]);
                let result = parse_dispatched(schema, pv13(), false);
                assert!(
                    result.is_err(),
                    "{key}: {value} must be rejected by the structural path at PV13 — the \
                     meta-schema does not run here, so this is the only gate"
                );
                let msg = format!("{:?}", result.unwrap_err());
                assert!(
                    msg.contains("unexpected property name"),
                    "PV13 must reject it as an unknown index key, exactly as a node without \
                     the feature does; got {msg}"
                );
            }
        }
    }

    /// Same schema, PV14, no full validation: accepted. Pins that the gate is
    /// the parser *generation* and not the validation mode.
    #[test]
    fn ranked_keywords_accepted_at_pv14_without_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema, pv14(), false)
            .expect("PV14 structural parse must accept the ranked keywords");
        assert!(
            v2.indices
                .get("byRestaurant")
                .expect("index parsed under its name")
                .ranked_averageable
        );
    }

    /// The meta-schema's ranked `if`/`then` conditionals are the declarative
    /// half of the structural "ranking needs its range axis" rule:
    /// `rankedCountable: true` without `rangeCountable` fails meta
    /// validation at PV14.
    #[test]
    fn ranked_countable_without_range_countable_rejected_by_meta_schema() {
        // `averageable` + `rangeAverageable` give the index its range axes in
        // *effect*, but `rangeCountable` is not literally present, so the
        // `if rankedCountable == true then require rangeCountable`
        // conditional fails.
        let schema = ranked_review_schema(vec![("rankedCountable", true)]);
        let result = parse_with(schema, pv14(), true);
        assert!(
            result.is_err(),
            "meta-schema v3 must demand rangeCountable alongside a true \
             rankedCountable"
        );
    }

    /// An index over `restaurantId` carrying exactly one ranked keyword and
    /// no aggregate layout whatsoever — the shape that separates "the key is
    /// present" from "a ranking axis was asked for".
    fn bare_ranked_index_schema(key: &str, value: bool) -> Value {
        let index_entry = Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("restaurantId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
            (Value::Text(key.to_string()), Value::Bool(value)),
        ]);

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![Value::Text("restaurantId".to_string())]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![index_entry]),
            ),
        ])
    }

    /// The ranked prerequisites are **value-sensitive**. `dependentRequired`
    /// fires on key *presence*, so expressing the opt-out explicitly
    /// (`"rankedCountable": false`) would have been made to demand a range
    /// axis the index never uses — a contract that says "no ranking here"
    /// rejected for not declaring the machinery of a ranking it declined.
    /// The structural parser reads `false` as "no ranking axis"; full
    /// validation at PV14 must agree.
    #[test]
    fn ranked_flags_written_out_as_false_do_not_require_a_range_axis() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            let v2 = parse_with(bare_ranked_index_schema(key, false), pv14(), true).unwrap_or_else(
                |e| panic!("`{key}: false` is an opt-out and must pass full validation: {e:?}"),
            );

            let index = v2
                .indices
                .get("byRestaurant")
                .expect("index parsed under its name");
            assert!(
                !index.ranked_countable && !index.ranked_summable && !index.ranked_averageable,
                "`{key}: false` must leave every ranking axis off"
            );
            assert!(
                !index.range_countable && !index.range_summable,
                "`{key}: false` must not have conjured a range axis either"
            );
        }
    }

    /// The other half of the same rule: `true` without the matching range
    /// axis is still refused under full validation, on every one of the three
    /// axes. Making the prerequisite value-sensitive must not have made it
    /// toothless.
    #[test]
    fn ranked_flags_set_true_still_require_their_range_axis() {
        for key in ["rankedCountable", "rankedSummable", "rankedAverageable"] {
            let result = parse_with(bare_ranked_index_schema(key, true), pv14(), true);
            assert!(
                result.is_err(),
                "`{key}: true` with no range axis must be rejected under full validation"
            );
        }
    }

    /// Structural counterpart of the check above, on the path where no
    /// meta-schema runs: `rankedCountable` with neither `countable` nor
    /// `rangeCountable` in effect is rejected by the index parser itself.
    #[test]
    fn ranked_countable_without_range_countable_rejected_structurally() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "restaurantId": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0,
                },
            },
            "required": ["restaurantId"],
            "additionalProperties": false,
            "indices": [{
                "name": "byRestaurant",
                "properties": [{ "restaurantId": "asc" }],
                "rankedCountable": true,
            }],
        });
        let result = parse_with(schema, pv14(), false);
        assert!(
            result.is_err(),
            "rankedCountable with no range-count layout must be rejected structurally"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rankedCountable") && msg.contains("rangeCountable"),
            "error must name both flags; got {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Ranked index key-length bounds
    // -----------------------------------------------------------------------
    //
    // A ranked index makes the property-name tree a grovedb *indexed* tree
    // whose children — the per-group value trees, keyed by the encoded
    // property value — are mirrored into an ordered secondary keyed by
    // `sort_key ‖ item_key`. Merk keys must stay below 256 bytes, so grovedb
    // caps the item key at `255 - sort_key_width`: 247 for the 8-byte Count
    // and Sum sort keys, 239 for the 16-byte Avg one. The generic index-key
    // limits (255 bytes / 63 characters) were derived against the bare 256
    // ceiling and are too loose by exactly the sort-key width, so the parser
    // has to tighten them or a contract would register and then abort the
    // batch of every document insert carrying an oversized key.
    //
    // The character bounds follow from the same worst case
    // `DocumentPropertyType::max_byte_size` uses — 4 UTF-8 bytes per
    // character, the very factor the generic 63 was derived from
    // (`floor(255 / 4)`):
    //
    //   Count/Sum: floor(247 / 4) = 61 characters (61 * 4 = 244 ✓, 62 * 4 = 248 ✗)
    //   Avg:       floor(239 / 4) = 59 characters (59 * 4 = 236 ✓, 60 * 4 = 240 ✗)

    /// A doctype whose indexed `restaurantId` property is given verbatim and
    /// whose single index carries `index_extras` on top of the mandatory
    /// name/properties pair. `grade` is always present so the summable and
    /// averageable layouts have an integer to aggregate.
    fn ranked_bound_schema(indexed_property: Value, index_extras: Vec<(&str, Value)>) -> Value {
        let mut index_entry: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("restaurantId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
        ];
        index_entry.extend(
            index_extras
                .into_iter()
                .map(|(key, value)| (Value::Text(key.to_string()), value)),
        );

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Map(vec![
                    (Value::Text("restaurantId".to_string()), indexed_property),
                    (
                        Value::Text("grade".to_string()),
                        platform_value!({
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100,
                            "position": 1,
                        }),
                    ),
                ]),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![
                    Value::Text("restaurantId".to_string()),
                    Value::Text("grade".to_string()),
                ]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![Value::Map(index_entry)]),
            ),
        ])
    }

    fn string_property(max_length: u32) -> Value {
        platform_value!({
            "type": "string",
            "maxLength": max_length,
            "position": 0,
        })
    }

    fn byte_array_property(max_items: u32) -> Value {
        platform_value!({
            "type": "array",
            "byteArray": true,
            "maxItems": max_items,
            "position": 0,
        })
    }

    /// Count axis only — an 8-byte sort key, so the 247-byte ceiling.
    fn count_ranked_extras() -> Vec<(&'static str, Value)> {
        vec![
            ("countable", Value::Text("countable".to_string())),
            ("rangeCountable", Value::Bool(true)),
            ("rankedCountable", Value::Bool(true)),
        ]
    }

    /// Sum axis only — also an 8-byte sort key, same 247-byte ceiling.
    fn sum_ranked_extras() -> Vec<(&'static str, Value)> {
        vec![
            ("summable", Value::Text("grade".to_string())),
            ("rangeSummable", Value::Bool(true)),
            ("rankedSummable", Value::Bool(true)),
        ]
    }

    /// Avg axis — a 16-byte sort key, so the tighter 239-byte ceiling.
    fn avg_ranked_extras() -> Vec<(&'static str, Value)> {
        vec![
            ("averageable", Value::Text("grade".to_string())),
            ("rangeAverageable", Value::Bool(true)),
            ("rankedAverageable", Value::Bool(true)),
        ]
    }

    fn parse_bound(schema: Value) -> Result<DocumentTypeV2, ProtocolError> {
        parse_with(schema, pv14(), true)
    }

    /// Count-ranked and sum-ranked indexes share the 8-byte sort key, so both
    /// stop the indexed string at 61 characters.
    #[test]
    fn ranked_count_and_sum_axes_bound_an_indexed_string_at_61_characters() {
        for (axis, extras) in [
            ("rankedCountable", count_ranked_extras()),
            ("rankedSummable", sum_ranked_extras()),
        ] {
            parse_bound(ranked_bound_schema(string_property(61), extras.clone()))
                .unwrap_or_else(|e| panic!("{axis}: 61 * 4 = 244 <= 247 must be accepted: {e:?}"));

            let error = parse_bound(ranked_bound_schema(string_property(62), extras))
                .expect_err("62 * 4 = 248 > 247 must be rejected");
            let msg = format!("{error:?}");
            assert!(
                msg.contains("maxLength") && msg.contains("61") && msg.contains("247"),
                "{axis}: the error must name maxLength, the 61-character bound and the \
                 247-byte key ceiling it derives from; got {msg}"
            );
        }
    }

    /// The Avg axis's 16-byte sort key costs 8 more bytes, and the string
    /// bound drops to 59 characters with it.
    #[test]
    fn ranked_avg_axis_bounds_an_indexed_string_at_59_characters() {
        parse_bound(ranked_bound_schema(
            string_property(59),
            avg_ranked_extras(),
        ))
        .expect("59 * 4 = 236 <= 239 must be accepted");

        let error = parse_bound(ranked_bound_schema(
            string_property(60),
            avg_ranked_extras(),
        ))
        .expect_err("60 * 4 = 240 > 239 must be rejected");
        let msg = format!("{error:?}");
        assert!(
            msg.contains("maxLength") && msg.contains("59") && msg.contains("239"),
            "the error must name maxLength, the 59-character bound and the 239-byte key \
             ceiling it derives from; got {msg}"
        );
    }

    /// A compound `[region, restaurantId]` avg-ranked index over the same
    /// doctype shape as [`ranked_bound_schema`], with independent control
    /// of the leading and terminal string properties' `maxLength`.
    fn compound_ranked_bound_schema(leading: Value, terminal: Value) -> Value {
        let mut index_entry: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRegionRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    Value::Map(vec![(
                        Value::Text("region".to_string()),
                        Value::Text("asc".to_string()),
                    )]),
                    Value::Map(vec![(
                        Value::Text("restaurantId".to_string()),
                        Value::Text("asc".to_string()),
                    )]),
                ]),
            ),
        ];
        index_entry.extend(
            avg_ranked_extras()
                .into_iter()
                .map(|(key, value)| (Value::Text(key.to_string()), value)),
        );

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Map(vec![
                    (Value::Text("region".to_string()), leading),
                    (Value::Text("restaurantId".to_string()), terminal),
                    (
                        Value::Text("grade".to_string()),
                        platform_value!({
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100,
                            "position": 2,
                        }),
                    ),
                ]),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![
                    Value::Text("region".to_string()),
                    Value::Text("restaurantId".to_string()),
                    Value::Text("grade".to_string()),
                ]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text("indices".to_string()),
                Value::Array(vec![Value::Map(index_entry)]),
            ),
        ])
    }

    fn string_property_at(max_length: u32, position: u32) -> Value {
        platform_value!({
            "type": "string",
            "maxLength": max_length,
            "position": position,
        })
    }

    /// The ranked ceiling binds only the **terminal** property — the one
    /// whose encoded value becomes the indexed tree's item key. A leading
    /// prefix property is an ordinary grovedb path segment: the generic
    /// 63-character indexed-string limit is what binds it, not the
    /// 59-character avg-ranked ceiling.
    #[test]
    fn ranked_ceiling_binds_the_terminal_property_not_the_leading_prefix() {
        // Leading at 63 (over the 59 ranked bound, at the generic bound)
        // with a terminal that fits the ranked bound: accepted.
        parse_bound(compound_ranked_bound_schema(
            string_property_at(63, 0),
            string_property_at(59, 1),
        ))
        .expect("a 63-character leading prefix is a path segment, not an item key");

        // The same 60-character string the single-property tests reject is
        // still rejected when it is the terminal property of a compound
        // ranked index.
        let error = parse_bound(compound_ranked_bound_schema(
            string_property_at(30, 0),
            string_property_at(60, 1),
        ))
        .expect_err("the terminal property keeps the 239-byte avg ceiling");
        let msg = format!("{error:?}");
        assert!(
            msg.contains("maxLength") && msg.contains("59") && msg.contains("239"),
            "the error must still name the ranked bound for the terminal; got {msg}"
        );

        // And the generic indexed-string limit still binds the leading
        // property: 64 characters is over the 63-character cap whether or
        // not the index is ranked.
        parse_bound(compound_ranked_bound_schema(
            string_property_at(64, 0),
            string_property_at(59, 1),
        ))
        .expect_err("the generic 63-character limit still binds the leading prefix");
    }

    /// Avg is strictly tighter than Count/Sum, and an index carrying Avg
    /// *alongside* the other axes has to satisfy the tightest of them: the
    /// 60-character string that a count-only ranked index accepts is refused
    /// the moment the Avg axis joins.
    #[test]
    fn the_avg_axis_bound_wins_when_several_ranking_axes_are_declared() {
        // Count-only accepts it...
        parse_bound(ranked_bound_schema(
            string_property(60),
            count_ranked_extras(),
        ))
        .expect("60 characters fits the count axis's 247-byte ceiling");

        // ...and adding the Avg axis to the same index refuses it.
        // The meta-schema's `dependentRequired` chain is literal, so
        // `rangeCountable` has to bring `countable` along explicitly even
        // though `averageable` already implies it in effect.
        let mut both = avg_ranked_extras();
        both.push(("countable", Value::Text("countable".to_string())));
        both.push(("rangeCountable", Value::Bool(true)));
        both.push(("rankedCountable", Value::Bool(true)));
        let error = parse_bound(ranked_bound_schema(string_property(60), both))
            .expect_err("the Avg axis's 239-byte ceiling must win over the count axis's 247");
        let msg = format!("{error:?}");
        assert!(
            msg.contains("239"),
            "the strictest axis's ceiling must be the one reported; got {msg}"
        );
    }

    /// Byte arrays are stored verbatim, so their bound is the ceiling itself:
    /// 247 bytes on the count/sum axes, 239 once Avg is in play.
    #[test]
    fn ranked_axes_bound_an_indexed_byte_array_at_the_raw_ceiling() {
        for (axis, extras, limit) in [
            ("rankedCountable", count_ranked_extras(), 247u32),
            ("rankedSummable", sum_ranked_extras(), 247),
            ("rankedAverageable", avg_ranked_extras(), 239),
        ] {
            parse_bound(ranked_bound_schema(
                byte_array_property(limit),
                extras.clone(),
            ))
            .unwrap_or_else(|e| panic!("{axis}: exactly {limit} bytes must be accepted: {e:?}"));

            let error =
                match parse_bound(ranked_bound_schema(byte_array_property(limit + 1), extras)) {
                    Ok(_) => panic!("{axis}: {} bytes must be rejected", limit + 1),
                    Err(error) => error,
                };
            let msg = format!("{error:?}");
            assert!(
                msg.contains("maxItems") && msg.contains(&limit.to_string()),
                "{axis}: the error must name maxItems and the {limit}-byte ceiling; got {msg}"
            );
        }
    }

    /// A property whose encoding is fixed and small — an integer group key is
    /// 8 bytes — can never reach any ranked ceiling, on any axis.
    #[test]
    fn a_fixed_width_indexed_property_always_fits_every_ranked_axis() {
        let integer_property = platform_value!({
            "type": "integer",
            "minimum": 0_u32,
            "maximum": 1_000_000_u32,
            "position": 0,
        });
        for (axis, extras) in [
            ("rankedCountable", count_ranked_extras()),
            ("rankedSummable", sum_ranked_extras()),
            ("rankedAverageable", avg_ranked_extras()),
        ] {
            parse_bound(ranked_bound_schema(integer_property.clone(), extras)).unwrap_or_else(
                |e| panic!("{axis}: a fixed-width integer group key must always fit: {e:?}"),
            );
        }
    }

    /// The tightening is scoped to ranked indexes. Without a ranking axis the
    /// generic limits are untouched — 63 characters and 255 bytes, both of
    /// which a ranked index would refuse.
    #[test]
    fn a_non_ranked_index_keeps_the_generic_key_limits() {
        parse_bound(ranked_bound_schema(string_property(63), vec![]))
            .expect("63 characters is the generic string limit and must still pass");
        let error = parse_bound(ranked_bound_schema(string_property(64), vec![]))
            .expect_err("64 characters exceeds the generic string limit");
        assert!(
            format!("{error:?}").contains("63"),
            "a non-ranked index must still report the generic 63-character limit"
        );

        parse_bound(ranked_bound_schema(byte_array_property(255), vec![]))
            .expect("255 bytes is the generic byte-array limit and must still pass");
        let error = parse_bound(ranked_bound_schema(byte_array_property(256), vec![]))
            .expect_err("256 bytes exceeds the generic byte-array limit");
        assert!(
            format!("{error:?}").contains("255"),
            "a non-ranked index must still report the generic 255-byte limit"
        );

        // The same aggregating (range) layout the ranking axes extend, minus
        // the ranking: still the generic limits, since no secondary is keyed
        // by these values.
        let mut range_only = avg_ranked_extras();
        range_only.retain(|(key, _)| *key != "rankedAverageable");
        parse_bound(ranked_bound_schema(string_property(63), range_only))
            .expect("a range-averageable index without a ranking axis keeps the generic limit");
    }

    // -------------------------------------------------------------------
    // Compound ranked indexes (per-prefix semantics) and the
    // prefix-overlap conflict
    // -------------------------------------------------------------------

    /// One extra single-property index for [`compound_ranked_schema`]:
    /// `(name, property, keys)`.
    type ExtraIndexSpec<'a> = (&'a str, &'a str, Vec<(&'a str, Value)>);

    /// A doctype with a compound ranked index `[region, restaurantId]`
    /// (avg axis on `grade`), plus optional extra single-property
    /// indexes to provoke — or fail to provoke — the prefix-overlap
    /// conflict.
    fn compound_ranked_schema(extra_indexes: Vec<ExtraIndexSpec>) -> Value {
        let compound_entry: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byRegionRestaurant".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    Value::Map(vec![(
                        Value::Text("region".to_string()),
                        Value::Text("asc".to_string()),
                    )]),
                    Value::Map(vec![(
                        Value::Text("restaurantId".to_string()),
                        Value::Text("asc".to_string()),
                    )]),
                ]),
            ),
            (
                Value::Text("averageable".to_string()),
                Value::Text("grade".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
            (
                Value::Text("rankedAverageable".to_string()),
                Value::Bool(true),
            ),
        ];
        let mut indices = vec![Value::Map(compound_entry)];
        for (name, property, keys) in extra_indexes {
            let mut entry: Vec<(Value, Value)> = vec![
                (
                    Value::Text("name".to_string()),
                    Value::Text(name.to_string()),
                ),
                (
                    Value::Text("properties".to_string()),
                    Value::Array(vec![Value::Map(vec![(
                        Value::Text(property.to_string()),
                        Value::Text("asc".to_string()),
                    )])]),
                ),
            ];
            entry.extend(
                keys.into_iter()
                    .map(|(key, value)| (Value::Text(key.to_string()), value)),
            );
            indices.push(Value::Map(entry));
        }

        Value::Map(vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "region": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 0,
                    },
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 1,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 2,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![
                    Value::Text("region".to_string()),
                    Value::Text("restaurantId".to_string()),
                    Value::Text("grade".to_string()),
                ]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (Value::Text("indices".to_string()), Value::Array(indices)),
        ])
    }

    /// A compound ranked index is accepted at PV14 with per-prefix
    /// semantics — on both the validating and the structural parse
    /// paths — and the ranked flags land on the index alongside the
    /// range axes they require.
    #[test]
    fn compound_ranked_index_accepted_at_pv14() {
        for full_validation in [true, false] {
            let v2 = parse_with(compound_ranked_schema(vec![]), pv14(), full_validation)
                .unwrap_or_else(|e| {
                    panic!(
                        "a compound ranked index must parse \
                         (full_validation: {full_validation}): {e}"
                    )
                });
            let index = v2
                .indices
                .get("byRegionRestaurant")
                .expect("index parsed under its name");
            assert!(index.ranked_averageable);
            assert!(index.range_countable && index.range_summable);
            assert_eq!(index.properties.len(), 2);
            assert_eq!(index.properties[1].name, "restaurantId");
        }
    }

    /// The one structurally impossible shape: a countable/summable
    /// index terminating at the compound ranked index's full leading
    /// prefix. The ranked terminal tree would sit inside aggregating
    /// value trees and need the NonCounted/NotSummed shell the storage
    /// layer rejects for indexed trees — so the contract is refused at
    /// parse time, on the validating AND the structural path (a
    /// contract smuggled through check_tx would brick document inserts).
    #[test]
    fn compound_ranked_with_aggregating_prefix_index_rejected() {
        let schema = compound_ranked_schema(vec![(
            "byRegion",
            "region",
            vec![("countable", Value::Text("countable".to_string()))],
        )]);
        for full_validation in [true, false] {
            let error = parse_with(schema.clone(), pv14(), full_validation).expect_err(
                "an aggregating index on the ranked compound's full prefix must be rejected",
            );
            let message = format!("{error:?}");
            assert!(
                message.contains("byRegionRestaurant")
                    && message.contains("byRegion")
                    && message.contains("NonCounted"),
                "the rejection must name both indexes and the structural conflict \
                 (full_validation: {full_validation}); got {message}"
            );
        }
    }

    /// Only the **exact** leading prefix conflicts: an aggregating
    /// index over a different property — same arity as the prefix, but
    /// not the prefix — coexists with the compound ranked index, as
    /// does a plain (non-aggregating) index on the prefix property.
    #[test]
    fn compound_ranked_with_non_conflicting_indexes_accepted() {
        // Countable over the *trailing* property's own single-property
        // index: terminates at [restaurantId], not at the ranked
        // index's [region] prefix.
        let aggregating_elsewhere = compound_ranked_schema(vec![(
            "byRestaurant",
            "restaurantId",
            vec![("countable", Value::Text("countable".to_string()))],
        )]);
        parse_with(aggregating_elsewhere, pv14(), true)
            .expect("an aggregating index off the prefix must not conflict");

        // A plain index on the prefix property: terminates at [region]
        // but carries no aggregates, so its value trees stay normal and
        // no wrapper shell is ever needed.
        let plain_prefix = compound_ranked_schema(vec![("byRegion", "region", vec![])]);
        parse_with(plain_prefix, pv14(), true)
            .expect("a non-aggregating index on the prefix must not conflict");
    }
}
