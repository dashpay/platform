use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_value_error;
use crate::data_contract::document_type::property::DocumentPropertyType;
use crate::data_contract::document_type::property_names::{
    DOCUMENTS_AVERAGEABLE, DOCUMENTS_COUNTABLE, DOCUMENTS_SUMMABLE, RANGE_AVERAGEABLE,
    RANGE_COUNTABLE, RANGE_SUMMABLE,
};
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentTypeV2 {
    /// Parses a document type schema with V2-specific fields (`documentsCountable`,
    /// `rangeCountable`). Delegates core parsing to the V1 parser, then wraps the
    /// result in a `DocumentTypeV2` with the additional fields set.
    ///
    /// This parser is only reachable from protocol version 12+ (via CONTRACT_VERSIONS_V4).
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
        // Extract V2-specific fields before the V1 parser consumes the schema map.
        //
        // Note on pre-v12 contracts: contracts created before v12 used the v1 parser
        // which ignores these fields. After v12 upgrade, deserialization uses the v2
        // parser which will read them. This is safe because the contract update path
        // runs through the v2 parser with full_validation=true, and the primary key
        // tree type is set correctly at contract creation time. Pre-v12 contracts
        // can only have these flags if they were explicitly set in the schema — the
        // meta-schema allows them as optional boolean properties.
        let schema_map_opt = schema.to_map().ok();

        let documents_countable = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, DOCUMENTS_COUNTABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?
            .unwrap_or(false);

        // Keep the raw `Option<bool>` so the averageable desugar below
        // can distinguish "field absent (default false)" from
        // "field explicit false" — same explicit-vs-default tracking
        // the Index parser does for its range axes. `range_countable`
        // (the resolved bool) flows into the rest of the logic.
        let range_countable_opt = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_COUNTABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?;
        let range_countable = range_countable_opt.unwrap_or(false);

        // `documentsSummable` names the integer property whose values are
        // summed across all documents of this type. When set, the primary
        // key tree is a `SumTree` (or `ProvableSumTree` if `rangeSummable`
        // is also true). Accepted shapes:
        //   - absent / null → no sum tree
        //   - non-empty string → property name
        //   - empty string → rejected (ValueWrongType)
        let documents_summable: Option<String> = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                schema_map
                    .iter()
                    .find(|(k, _)| k.as_text() == Some(DOCUMENTS_SUMMABLE))
            })
            .map(|(_, v)| match v {
                Value::Null => Ok(None),
                Value::Text(s) if !s.is_empty() => Ok(Some(s.clone())),
                Value::Text(_) => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsSummable must be a non-empty string naming an integer \
                         property, or null"
                            .to_string(),
                    ),
                )),
                _ => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsSummable value must be a string or null".to_string(),
                    ),
                )),
            })
            .transpose()?
            .flatten();

        let range_summable_opt = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_SUMMABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?;
        let range_summable = range_summable_opt.unwrap_or(false);

        // `documentsAverageable` is syntactic sugar for
        // `documentsCountable: true` + `documentsSummable: "<prop>"`.
        // `rangeAverageable` is shorthand for both range_* flags.
        // Both desugar into the underlying flags below.
        let documents_averageable: Option<String> = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                schema_map
                    .iter()
                    .find(|(k, _)| k.as_text() == Some(DOCUMENTS_AVERAGEABLE))
            })
            .map(|(_, v)| match v {
                Value::Null => Ok(None),
                Value::Text(s) if !s.is_empty() => Ok(Some(s.clone())),
                Value::Text(_) => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsAverageable must be a non-empty string naming an integer \
                         property, or null"
                            .to_string(),
                    ),
                )),
                _ => Err(ProtocolError::DataContractError(
                    DataContractError::ValueWrongType(
                        "documentsAverageable value must be a string or null".to_string(),
                    ),
                )),
            })
            .transpose()?
            .flatten();

        let range_averageable = schema_map_opt
            .as_ref()
            .and_then(|schema_map| {
                Value::inner_optional_bool_value(schema_map, RANGE_AVERAGEABLE)
                    .map_err(consensus_or_protocol_value_error)
                    .transpose()
            })
            .transpose()?
            .unwrap_or(false);

        // Desugar averageable into count + sum flags. Conflict rules
        // mirror the per-index dispatch: if both `averageable` and
        // `documentsSummable` are set, the property names must match;
        // `documentsCountable: false` alongside `averageable` is a
        // contradiction.
        let (documents_countable, documents_summable, range_countable, range_summable) =
            if let Some(avg_prop) = &documents_averageable {
                if let Some(sum_prop) = &documents_summable {
                    if sum_prop != avg_prop {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "documentsAverageable=\"{}\" conflicts with \
                                 documentsSummable=\"{}\" on document type \"{}\": both name \
                                 the property aggregated into the primary-key sum tree, so \
                                 they must agree (or set only one — documentsAverageable is \
                                 shorthand for documentsCountable + documentsSummable on the \
                                 same property)",
                                avg_prop, sum_prop, name,
                            )),
                        ));
                    }
                }
                // averageable implies countable; explicit
                // `documentsCountable: false` alongside is a contradiction.
                if let Some(schema_map) = schema_map_opt.as_ref() {
                    if let Some(explicit_countable) =
                        Value::inner_optional_bool_value(schema_map, DOCUMENTS_COUNTABLE)
                            .map_err(consensus_or_protocol_value_error)?
                    {
                        if !explicit_countable {
                            return Err(ProtocolError::DataContractError(
                                DataContractError::InvalidContractStructure(format!(
                                    "documentsAverageable=\"{}\" on document type \"{}\" \
                                     implies documentsCountable: true, but the schema \
                                     explicitly sets documentsCountable: false. Remove the \
                                     explicit false (or drop documentsAverageable in favor \
                                     of just documentsSummable).",
                                    avg_prop, name,
                                )),
                            ));
                        }
                    }
                }
                // When `rangeAverageable: true` is set, BOTH range axes
                // are promoted. Reject explicit-`false` contradictions
                // on either axis (silently flipping the author's
                // explicit value would emit the wrong on-disk layout).
                // Omitted / default-false → silently promoted.
                if range_averageable {
                    if range_countable_opt == Some(false) {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "rangeAverageable: true on document type \"{}\" conflicts \
                                 with explicit rangeCountable: false: rangeAverageable is \
                                 shorthand for rangeCountable + rangeSummable on the \
                                 averageable property. Remove the explicit \
                                 `rangeCountable: false` (or drop rangeAverageable in \
                                 favor of rangeSummable alone).",
                                name,
                            )),
                        ));
                    }
                    if range_summable_opt == Some(false) {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "rangeAverageable: true on document type \"{}\" conflicts \
                                 with explicit rangeSummable: false: rangeAverageable is \
                                 shorthand for rangeCountable + rangeSummable on the \
                                 averageable property. Remove the explicit \
                                 `rangeSummable: false` (or drop rangeAverageable in favor \
                                 of rangeCountable alone).",
                                name,
                            )),
                        ));
                    }
                }
                // Promote each range axis independently: `rangeAverageable`
                // (shorthand) sets BOTH; explicit `rangeCountable` /
                // `rangeSummable` only set their own axis. Mirrors the
                // per-index parser at `index/mod.rs` (search for
                // `if range_averageable {`) — without this split, the
                // shorthand `documentsAverageable + rangeSummable: true`
                // would silently flip `range_countable` to true, which
                // diverges from the longhand `documentsCountable +
                // documentsSummable + rangeSummable: true` form
                // (`range_countable` stays false there) and emits a
                // different on-disk tree shape than the author asked
                // for.
                let merged_range_countable = range_countable || range_averageable;
                let merged_range_summable = range_summable || range_averageable;
                (
                    true,
                    Some(avg_prop.clone()),
                    merged_range_countable,
                    merged_range_summable,
                )
            } else if range_averageable {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "rangeAverageable: true on document type \"{}\" requires \
                         documentsAverageable: \"<prop>\" to name the integer property to \
                         average; rangeAverageable on its own has no property to aggregate",
                        name,
                    )),
                ));
            } else {
                (
                    documents_countable,
                    documents_summable,
                    range_countable,
                    range_summable,
                )
            };

        // Cross-validation: `rangeSummable: true` requires
        // `documentsSummable` to be set. (Mirrors count's
        // `rangeCountable implies documentsCountable` rule at the
        // doctype level.) This also catches the
        // `rangeAverageable + no documentsAverageable + no documentsSummable`
        // case above, but the earlier explicit error gives a better
        // message for the averageable-specific path.
        if range_summable && documents_summable.is_none() {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidContractStructure(
                    "rangeSummable: true requires documentsSummable to name an integer \
                     property; range-sum queries on the primary key only make sense on \
                     a sum-bearing doctype"
                        .to_string(),
                ),
            ));
        }

        // Delegate core parsing to V1
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

        // Convert to V2 and set the new fields
        let mut v2: DocumentTypeV2 = v1.into();
        v2.documents_countable = documents_countable || range_countable;
        v2.range_countable = range_countable;
        v2.documents_summable = documents_summable.clone();
        v2.range_summable = range_summable;

        // `documentsKeepHistory: true` + `documentsSummable: <prop>` IS
        // supported (as of the keep-history sum-aware-reference change).
        // Layout: the per-document subtree at `[..doctype, doc_id]`
        // becomes a `SumTree` (was `NormalTree`); the version bodies
        // under `[..doctype, doc_id, t_N]` stay plain `Item`s (NOT
        // `ItemWithSumItem`) so historical versions don't double-count;
        // the `[..doctype, doc_id, 0]` "current pointer" becomes a
        // `ReferenceWithSumItem` carrying the current version's
        // `sum_property` value. Aggregation walks:
        //
        //   - Per-doc SumTree aggregate = `0`-key's sum_value (= current
        //     version's amount) + 0 from each history Item. Result: the
        //     current version's contribution.
        //   - Doctype-level SumTree aggregate = sum over per-doc SumTree
        //     aggregates = total of CURRENT versions across all docs.
        //
        // On update, rewriting the `0`-key reference with the new
        // version's sum_value triggers grovedb's standard
        // delete-then-insert merk propagation, which carries the delta
        // up to ancestors automatically. No separate shadow tree or
        // parallel bookkeeping. Same `Element::ReferenceWithSumItem`
        // primitive the per-index sum-tree path already uses (see
        // `make_document_reference_with_sum_item` on the rs-drive side).

        // Cross-validate: every index with `summable` set must name the
        // same property as `documents_summable` (if doctype-level
        // summable is set). Reason: grovedb sum trees aggregate `i64`
        // per merk node — there's no per-tree property tag, so all sum
        // contributions feeding into a doctype's storage must come from
        // the same document property. If one index claimed
        // `summable: "fee"` while another claimed `summable: "amount"`
        // they'd both write `ItemWithSumItem` contributions into the
        // same merk hierarchy and produce a meaningless aggregation.
        //
        // We also enforce this when `documents_summable` is unset: in
        // that case every per-index `summable` must agree with all
        // other per-index `summable`s (the first one wins as the
        // canonical name).
        //
        // These checks are structural invariants of the on-disk
        // grovedb sum-tree layout, NOT optional schema lints — mixed
        // sum properties corrupt ancestor aggregation, U64 summable
        // values silently overflow grovedb's `i64` SumValue at insert,
        // and non-required summable properties silently underflow
        // ancestor sums on delete. They run regardless of
        // `full_validation` because this function sits on the
        // untrusted-contract boundary (restore / migration /
        // cache-warmup / future query-side parsing paths may pass
        // `full_validation: false` against attacker-controlled
        // contract bytes — admitting malformed contracts there would
        // let SUM/AVG queries compute over meaningless state while
        // still looking structurally valid). `flattened_properties`
        // and `required_fields` are populated by the V1 parser on
        // both validation paths so the lookups below are safe to
        // execute unconditionally.
        let mut canonical: Option<String> = documents_summable.clone();
        for index in v2.indices.values() {
            if let Some(index_sum_property) = &index.summable {
                match &canonical {
                    Some(existing) if existing != index_sum_property => {
                        return Err(ProtocolError::DataContractError(
                            DataContractError::InvalidContractStructure(format!(
                                "all `summable` declarations on document type \"{}\" \
                                 must name the same property; saw \"{}\" and \"{}\". \
                                 Sum trees aggregate i64 per merk node and have no \
                                 per-tree property tag — mixed sum properties would \
                                 produce a meaningless aggregation.",
                                name, existing, index_sum_property,
                            )),
                        ));
                    }
                    None => canonical = Some(index_sum_property.clone()),
                    _ => {}
                }
            }
        }

        // Also verify the named property is `type: integer` and
        // listed in `required`. The integer check goes through
        // `v2.flattened_properties` (set by the V1 parser, which
        // resolves $ref). The required check goes through
        // `v2.required_fields`.
        if let Some(prop_name) = &canonical {
            let prop = v2.flattened_properties.get(prop_name).ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                    format!(
                        "summable property \"{}\" referenced by document type \"{}\" \
                         does not exist on that document type",
                        prop_name, name,
                    ),
                ))
            })?;
            // U64 is intentionally NOT accepted: grovedb's sum-tree
            // aggregates `i64`, so a u64 value > i64::MAX would
            // overflow the aggregator silently. Authors who want
            // unbounded positive integers as summable should set
            // the schema's `maximum` explicitly to `i64::MAX`
            // (9_223_372_036_854_775_807) — that bound forces the
            // property-type inference at
            // `property/mod.rs::find_unsigned_integer_type_for_max_value`
            // through `find_integer_type_for_min_and_max_values`'s
            // unsigned branch (still U64 today because max > U32),
            // BUT we also reject U64 unconditionally here so the
            // rule is enforced regardless of the inference path.
            //
            // The accepted list (I64 + I32/U32 + I16/U16 + I8/U8) is
            // the set of integer types that fit losslessly into
            // grovedb's i64 sum value. Without an explicit `maximum
            // <= i64::MAX` on the property, no integer schema
            // currently infers I64 — authors must add either
            // `maximum: 9223372036854775807` or pick a smaller
            // signed/unsigned type that's not U64.
            if !matches!(
                prop.property_type,
                DocumentPropertyType::I64
                    | DocumentPropertyType::I32
                    | DocumentPropertyType::U32
                    | DocumentPropertyType::I16
                    | DocumentPropertyType::U16
                    | DocumentPropertyType::I8
                    | DocumentPropertyType::U8
            ) {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "summable property \"{}\" on document type \"{}\" must be an \
                         integer type whose values fit in i64 (i8..i64 / u8..u32); got \
                         {:?}. U64 is rejected because values above i64::MAX would \
                         overflow grovedb's i64 sum aggregator. To use a positive-only \
                         integer property as summable, either pick u8/u16/u32, OR set the \
                         property's schema `maximum` to 9223372036854775807 (i64::MAX) \
                         AND have it parse as i64 (today this requires a negative \
                         `minimum` to force the signed inference branch; tracked as a \
                         property-inference follow-up).",
                        prop_name, name, prop.property_type,
                    )),
                ));
            }
            if !v2.required_fields.contains(prop_name) {
                return Err(ProtocolError::DataContractError(
                    DataContractError::InvalidContractStructure(format!(
                        "summable property \"{}\" on document type \"{}\" must be \
                         listed in the document type's `required` array; a missing \
                         value at insert time would leave the reference with no sum \
                         contribution and silently underflow ancestor sums on delete.",
                        prop_name, name,
                    )),
                ));
            }
        }

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
        parse_with(schema, PlatformVersion::latest(), true)
    }

    /// `parse` with the platform version and validation mode spelled out —
    /// needed by the ranked-keyword gating tests, which have to exercise both
    /// the meta-schema path (`full_validation: true`) and the structural path
    /// (`full_validation: false`) on both sides of the PV14 boundary.
    fn parse_with(
        schema: Value,
        platform_version: &PlatformVersion,
        full_validation: bool,
    ) -> Result<DocumentTypeV2, ProtocolError> {
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on this platform version");
        DocumentTypeV2::try_from_schema(
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

    // -----------------------------------------------------------------------
    // Ranked aggregate index keywords — version gating
    //
    // The keywords live on the *index*, not the document type, but the gate
    // that admits them is the doctype's `document_type_schema` version (2 at
    // PV13, 3 at PV14), so the tests belong on this parser. Two independent
    // halves have to hold on the pre-PV14 side:
    //
    //   * `full_validation: true`  — the v2 meta-schema has
    //     `additionalProperties: false` on index entries and rejects the key.
    //   * `full_validation: false` — no meta-schema runs at all (check_tx,
    //     cache warm-up, restore), so the *grammar* itself must not know the
    //     keyword. That is the smuggling path, and the one worth pinning.
    // -----------------------------------------------------------------------

    /// A `review` doctype with one index over `restaurantId`, averageable on
    /// `grade`, optionally carrying ranked keywords. Written so the v3
    /// meta-schema's `dependentRequired` chain is satisfied
    /// (`rankedAverageable` → `rangeAverageable` → `averageable`).
    fn ranked_review_schema(ranked_keys: Vec<(&str, bool)>) -> Value {
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
                        "maxLength": 63,
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

    /// PV14 (document meta-schema v3) accepts the ranked keywords and carries
    /// them onto the parsed index.
    #[test]
    fn ranked_keywords_accepted_at_pv14() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema, PlatformVersion::latest(), true)
            .expect("meta-schema v3 must accept the ranked index keywords");

        let index = v2
            .indices
            .get("byRestaurant")
            .expect("index parsed under its name");
        assert!(index.ranked_averageable);
        assert!(!index.ranked_countable);
        assert!(!index.ranked_summable);
        assert!(index.range_countable && index.range_summable);
    }

    /// PV13 + `full_validation`: the v2 meta-schema rejects the unknown index
    /// key outright.
    #[test]
    fn ranked_keywords_rejected_at_pv13_under_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let result = parse_with(schema, pv13(), true);
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
                let result = parse_with(schema, pv13(), false);
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
    /// the schema *version* and not the validation mode.
    #[test]
    fn ranked_keywords_accepted_at_pv14_without_full_validation() {
        let schema = ranked_review_schema(vec![("rankedAverageable", true)]);
        let v2 = parse_with(schema, PlatformVersion::latest(), false)
            .expect("PV14 structural parse must accept the ranked keywords");
        assert!(
            v2.indices
                .get("byRestaurant")
                .expect("index parsed under its name")
                .ranked_averageable
        );
    }

    /// The meta-schema's `dependentRequired` is the declarative half of the
    /// structural "ranking needs its range axis" rule: `rankedCountable`
    /// without `rangeCountable` fails meta validation at PV14.
    #[test]
    fn ranked_countable_without_range_countable_rejected_by_meta_schema() {
        // `averageable` + `rangeAverageable` give the index its range axes in
        // *effect*, but `rangeCountable` is not literally present, so
        // `dependentRequired: {rankedCountable: ["rangeCountable"]}` fails.
        let schema = ranked_review_schema(vec![("rankedCountable", true)]);
        let result = parse_with(schema, PlatformVersion::latest(), true);
        assert!(
            result.is_err(),
            "meta-schema v3 dependentRequired must demand rangeCountable alongside \
             rankedCountable"
        );
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
        let result = parse_with(schema, PlatformVersion::latest(), false);
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
}
