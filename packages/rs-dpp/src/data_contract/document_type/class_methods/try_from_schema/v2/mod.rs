//! Document-type parser **generation 2** — protocol versions 12 and 13.
//!
//! Generation 2 is generation 1 plus the doctype-level aggregate keywords
//! (`documentsCountable` / `rangeCountable` / `documentsSummable` /
//! `rangeSummable` and the `documentsAverageable` / `rangeAverageable`
//! shorthands). Core parsing is delegated to the generation-1 entry point,
//! which is where the `document_type_schema` read that distinguishes PV12 from
//! PV13 lives; the aggregate keywords themselves do not vary across the two, so
//! this module passes no version-dependent constants at all.

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use super::common;

// Reached by this module's `#[cfg(test)] mod tests` through `use super::*`.
#[cfg(test)]
use crate::data_contract::document_type::property_names::{
    DOCUMENTS_AVERAGEABLE, DOCUMENTS_SUMMABLE, RANGE_AVERAGEABLE, RANGE_COUNTABLE, RANGE_SUMMABLE,
};

impl DocumentTypeV2 {
    /// Parses a document type schema with the doctype-level aggregate fields,
    /// then wraps the parsed core in a `DocumentTypeV2` with those fields set.
    ///
    /// This parser is only reachable from protocol version 12+ (via
    /// CONTRACT_VERSIONS_V4).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_from_schema(
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
        // Read the aggregate keywords before the core parser consumes `schema`.
        let aggregates = common::parse_doctype_aggregate_keywords(&schema, name)?;

        // Delegate core parsing to generation 1. Going through generation 1's
        // entry point rather than straight to the shared core is deliberate:
        // that entry point owns the `document_type_schema` read which decides
        // whether the `keeps*History` flags are part of the grammar, and that
        // decision differs between the two protocol versions this generation
        // serves (absent at PV12, present at PV13).
        let v1 = DocumentTypeV1::try_from_schema(
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
        )?;

        // Convert to V2 and set the aggregate fields
        let mut v2: DocumentTypeV2 = v1.into();
        common::apply_doctype_aggregates(&mut v2, aggregates, name)?;

        Ok(v2)
    }
}

impl DocumentType {
    /// Dispatches to `DocumentTypeV2::try_from_schema` and wraps the result.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract::document_type::class_methods) fn try_from_schema_v2(
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
        DocumentTypeV2::try_from_schema(
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
mod tests {
    //! Regression tests for the doctype-level `rangeAverageable`
    //! contradiction guards added next to the per-index ones in
    //! `index/mod.rs`. Mirror of
    //! `test_index_try_from_range_averageable_with_explicit_range_*_false_rejected`
    //! at the document-type-schema level: same vector (the explicit
    //! `false` being silently flipped to `true`), same expected
    //! rejection shape (`InvalidContractStructure` naming the
    //! conflicting flag), but exercising the parser at
    //! `DocumentTypeV2::try_from_schema` rather than at the per-index
    //! `Index::try_from` boundary.
    use super::*;
    use platform_value::platform_value;

    /// Generation-specific tests must pin a protocol version that actually
    /// selects their own generation: `PlatformVersion::latest()` silently
    /// retargets these tests onto a different parser generation and a
    /// different document meta-schema whenever LATEST moves. PV13 is the
    /// highest protocol version whose `try_from_schema` selects generation 2,
    /// and its `document_type_schema` is 2 — the same meta-schema `latest()`
    /// resolved to before generation 3 existed, so behavior here is unchanged.
    fn generation_2_platform_version() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("protocol version 13 exists")
    }

    /// Build a minimal v2-shaped document-type schema with
    /// `documentsAverageable: "score"` and the supplied
    /// `rangeAverageable` / `rangeCountable` / `rangeSummable`
    /// values. `score` is the canonical summable property
    /// (integer with `minimum`/`maximum` bounding it inside `i64`,
    /// listed in `required` — both invariants the structural
    /// summable checks enforce).
    fn build_schema(
        range_averageable: Option<bool>,
        range_countable: Option<bool>,
        range_summable: Option<bool>,
    ) -> Value {
        let mut schema_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("type".to_string()),
                Value::Text("object".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                platform_value!({
                    "score": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 0,
                    },
                }),
            ),
            (
                Value::Text("required".to_string()),
                Value::Array(vec![Value::Text("score".to_string())]),
            ),
            (
                Value::Text("additionalProperties".to_string()),
                Value::Bool(false),
            ),
            (
                Value::Text(DOCUMENTS_AVERAGEABLE.to_string()),
                Value::Text("score".to_string()),
            ),
        ];
        if let Some(b) = range_averageable {
            schema_map.push((Value::Text(RANGE_AVERAGEABLE.to_string()), Value::Bool(b)));
        }
        if let Some(b) = range_countable {
            schema_map.push((Value::Text(RANGE_COUNTABLE.to_string()), Value::Bool(b)));
        }
        if let Some(b) = range_summable {
            // The doctype-level meta schema declares
            // `dependentRequired: { rangeSummable: ["documentsSummable"] }`
            // — once `rangeSummable` is present (true OR false) the
            // schema demands `documentsSummable` too. Since
            // `documentsAverageable: "score"` already implies it (and
            // any explicit `documentsSummable` must match per the
            // parser's cross-check), add it redundantly so the schema
            // passes meta validation when the test exercises a
            // `rangeSummable` value at all. Same redundancy `grades`
            // / `tip-jar` contract fixtures use.
            schema_map.push((
                Value::Text(DOCUMENTS_SUMMABLE.to_string()),
                Value::Text("score".to_string()),
            ));
            schema_map.push((Value::Text(RANGE_SUMMABLE.to_string()), Value::Bool(b)));
        }
        Value::Map(schema_map)
    }

    fn parse(schema: Value) -> Result<DocumentTypeV2, ProtocolError> {
        let platform_version = generation_2_platform_version();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on latest platform version");
        DocumentTypeV2::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test_doc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
    }

    /// `documentsAverageable: "score" + rangeAverageable: true +
    /// rangeCountable: false` — explicit-false on the count side
    /// contradicts the shorthand. Must reject.
    #[test]
    fn doctype_range_averageable_with_explicit_range_countable_false_rejected() {
        let schema = build_schema(Some(true), Some(false), None);
        let result = parse(schema);
        assert!(
            result.is_err(),
            "rangeAverageable: true + rangeCountable: false must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rangeAverageable") && msg.contains("rangeCountable"),
            "error must reference both rangeAverageable and rangeCountable; got {msg}"
        );
    }

    /// Sum-side analog: `rangeAverageable: true + rangeSummable: false`
    /// — same contradiction shape, must reject.
    #[test]
    fn doctype_range_averageable_with_explicit_range_summable_false_rejected() {
        let schema = build_schema(Some(true), None, Some(false));
        let result = parse(schema);
        assert!(
            result.is_err(),
            "rangeAverageable: true + rangeSummable: false must be rejected"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("rangeAverageable") && msg.contains("rangeSummable"),
            "error must reference both rangeAverageable and rangeSummable; got {msg}"
        );
    }

    /// `rangeAverageable: true + rangeCountable: true +
    /// rangeSummable: true` — redundant-but-consistent explicit
    /// `true` on both range axes must be accepted (the values
    /// agree with the shorthand's promotion).
    #[test]
    fn doctype_range_averageable_with_redundant_explicit_range_true_accepted() {
        let schema = build_schema(Some(true), Some(true), Some(true));
        let v2 = parse(schema).expect(
            "redundant-but-consistent explicit range flags must parse cleanly alongside \
             rangeAverageable: true",
        );
        assert!(
            v2.range_countable,
            "rangeAverageable should leave range_countable true"
        );
        assert!(
            v2.range_summable,
            "rangeAverageable should leave range_summable true"
        );
    }

    /// Canonical shorthand `documentsAverageable: "score" +
    /// rangeAverageable: true` (no explicit `rangeCountable` /
    /// `rangeSummable`) must succeed and silently promote both
    /// range axes — the "default-false → silently promoted" path
    /// that the explicit-false rejection guards have to leave
    /// intact.
    #[test]
    fn doctype_range_averageable_alone_silently_promotes_range_axes() {
        let schema = build_schema(Some(true), None, None);
        let v2 = parse(schema).expect("canonical rangeAverageable shorthand must parse");
        assert!(
            v2.range_countable,
            "rangeAverageable: true should promote range_countable when not explicit"
        );
        assert!(
            v2.range_summable,
            "rangeAverageable: true should promote range_summable when not explicit"
        );
    }

    /// `documentsKeepHistory: true + documentsSummable: "score"` is
    /// SUPPORTED. The rs-drive insert path materializes the per-doc
    /// subtree as a `SumTree`, writes version bodies as plain `Item`s
    /// (no `sum_value` so historical versions don't double-count),
    /// and writes a `ReferenceWithSumItem` at the `0`-key carrying
    /// the current version's `sum_property` value. Aggregation walks
    /// then deliver the current-versions-only sum at the doctype
    /// root.
    ///
    /// Earlier versions of this PR rejected this combination at parse
    /// time because we hadn't worked through the
    /// `ReferenceWithSumItem`-on-`0`-key approach; that rejection is
    /// gone, and this test pins that the combination parses cleanly
    /// AND that both flags survive into the parsed `v2`.
    #[test]
    fn doctype_keep_history_with_documents_summable_accepted() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsKeepHistory": true,
            "documentsSummable": "score",
        });
        let v2 = parse(schema).expect(
            "documentsKeepHistory: true + documentsSummable must be accepted (the per-doc \
             SumTree + ReferenceWithSumItem-on-0-key layout makes this combination correct)",
        );
        assert!(
            v2.documents_keep_history,
            "documentsKeepHistory: true must be carried into v2"
        );
        assert_eq!(
            v2.documents_summable.as_deref(),
            Some("score"),
            "documents_summable must be carried into v2"
        );
    }

    /// Same acceptance via the `documentsAverageable` shorthand
    /// (desugars to documentsCountable: true + documentsSummable on
    /// the same property). The sum half rides the same
    /// keep-history + sum-aware-reference layout; the count half
    /// composes through the doctype's primary-key tree being a
    /// `CountSumTree` / `ProvableCountSumTree` variant.
    #[test]
    fn doctype_keep_history_with_documents_averageable_accepted() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsKeepHistory": true,
            "documentsAverageable": "score",
        });
        let v2 = parse(schema).expect(
            "documentsKeepHistory + documentsAverageable must be accepted (same layout as \
             the documents_summable acceptance above)",
        );
        assert!(v2.documents_keep_history);
        // averageable desugars to countable + summable
        assert!(v2.documents_countable);
        assert_eq!(v2.documents_summable.as_deref(), Some("score"));
    }

    /// `documentsKeepHistory: true` WITHOUT any summable flag must
    /// continue to parse cleanly — only the combination is rejected.
    /// Guards against an over-aggressive predicate that would break
    /// every existing keep-history doctype.
    #[test]
    fn doctype_keep_history_without_summable_accepted() {
        let schema = platform_value!({
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
        });
        let v2 = parse(schema).expect("keep-history without summable must parse cleanly");
        assert!(
            v2.documents_keep_history,
            "documentsKeepHistory: true must be carried into v2"
        );
        assert!(
            v2.documents_summable.is_none(),
            "no summable flag set, documents_summable must be None"
        );
    }

    /// Shorthand `documentsAverageable: "score"` with
    /// `rangeSummable: true` (no `rangeAverageable`, no
    /// `rangeCountable`) must desugar to the SAME
    /// `(range_countable, range_summable)` pair as the longhand
    /// form combining `documentsCountable: true`,
    /// `documentsSummable: "score"`, and `rangeSummable: true`.
    /// Specifically: `range_countable: false` (no caller asked for
    /// it) and `range_summable: true`.
    ///
    /// Pre-fix, the doctype parser at `v2/mod.rs` merged the two
    /// range axes together (computing
    /// `range_countable || range_summable || range_averageable` for
    /// BOTH outputs), so the shorthand silently flipped
    /// `range_countable` to true. The longhand form leaves it
    /// false. That asymmetry made shorthand semantically distinct
    /// from its desugaring — emitting a different on-disk tree
    /// shape on the count axis than the author asked for.
    ///
    /// Mirrors the per-index parser at `index/mod.rs` (search for
    /// `if range_averageable {`), which only promotes both axes
    /// when `rangeAverageable: true` is set.
    #[test]
    fn doctype_documents_averageable_with_range_summable_matches_longhand() {
        // Shorthand: `documentsAverageable: "score" + rangeSummable: true`.
        let shorthand_schema = build_schema(None, None, Some(true));
        let shorthand =
            parse(shorthand_schema).expect("shorthand + rangeSummable: true must parse");

        // Longhand: explicit `documentsCountable: true + documentsSummable +
        // rangeSummable: true`. `build_schema` doesn't model this — write
        // the schema directly so the test is a faithful comparison.
        let longhand_schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsCountable": true,
            "documentsSummable": "score",
            "rangeSummable": true,
        });
        let longhand = parse(longhand_schema)
            .expect("longhand documentsCountable + documentsSummable + rangeSummable must parse");

        assert_eq!(
            (
                shorthand.documents_countable,
                shorthand.documents_summable.clone(),
                shorthand.range_countable,
                shorthand.range_summable,
            ),
            (
                longhand.documents_countable,
                longhand.documents_summable.clone(),
                longhand.range_countable,
                longhand.range_summable,
            ),
            "shorthand `documentsAverageable + rangeSummable: true` must produce the same \
             (documents_countable, documents_summable, range_countable, range_summable) tuple \
             as the longhand `documentsCountable + documentsSummable + rangeSummable: true`. \
             Pre-fix, the shorthand silently set range_countable=true while the longhand \
             left it false."
        );
        assert!(
            !shorthand.range_countable,
            "neither form requested rangeCountable; expected range_countable=false but got \
             true — the shorthand merge is leaking range_summable into the count axis"
        );
        assert!(
            shorthand.range_summable,
            "rangeSummable: true must carry through the shorthand desugar"
        );
    }

    /// Sum-axis mirror of the test above: shorthand
    /// `documentsAverageable + rangeCountable: true` must match the
    /// longhand `documentsCountable + documentsSummable +
    /// rangeCountable: true`. Pinning both directions so a future
    /// refactor can't accidentally re-introduce the leak on only one
    /// axis.
    #[test]
    fn doctype_documents_averageable_with_range_countable_matches_longhand() {
        let shorthand_schema = build_schema(None, Some(true), None);
        let shorthand =
            parse(shorthand_schema).expect("shorthand + rangeCountable: true must parse");

        let longhand_schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsCountable": true,
            "documentsSummable": "score",
            "rangeCountable": true,
        });
        let longhand = parse(longhand_schema)
            .expect("longhand documentsCountable + documentsSummable + rangeCountable must parse");

        assert_eq!(
            (
                shorthand.documents_countable,
                shorthand.documents_summable.clone(),
                shorthand.range_countable,
                shorthand.range_summable,
            ),
            (
                longhand.documents_countable,
                longhand.documents_summable.clone(),
                longhand.range_countable,
                longhand.range_summable,
            ),
            "shorthand `documentsAverageable + rangeCountable: true` must produce the same \
             tuple as the longhand `documentsCountable + documentsSummable + rangeCountable: \
             true`. Pre-fix the shorthand silently set range_summable=true."
        );
        assert!(
            shorthand.range_countable,
            "rangeCountable: true must carry through the shorthand desugar"
        );
        assert!(
            !shorthand.range_summable,
            "neither form requested rangeSummable; expected range_summable=false but got \
             true — the shorthand merge is leaking range_countable into the sum axis"
        );
    }

    /// Symmetric: `documentsSummable` on a NON-keep-history doctype
    /// stays valid. Guards against a rejection that triggers on
    /// summable alone instead of the AND.
    #[test]
    fn doctype_summable_without_keep_history_accepted() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsSummable": "score",
        });
        let v2 = parse(schema).expect("summable without keep-history must parse cleanly");
        assert!(
            !v2.documents_keep_history,
            "documentsKeepHistory absent must default to false"
        );
        assert_eq!(
            v2.documents_summable.as_deref(),
            Some("score"),
            "documents_summable must be carried into v2"
        );
    }
}
