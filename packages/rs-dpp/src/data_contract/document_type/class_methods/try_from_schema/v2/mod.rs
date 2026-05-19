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
                let merged_range = range_countable || range_summable || range_averageable;
                (true, Some(avg_prop.clone()), merged_range, merged_range)
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
