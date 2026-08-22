//! Ranked (top-K) and having-range document queries — the protocol
//! version 14 aggregate-ordering surface.
//!
//! Both modes ride the same `getDocuments` RPC as the count / sum /
//! average surface in [`super::document`], and both return the same
//! per-group entry shape. They differ in what bounds the page:
//!
//! - **ranked** — `ORDER BY <aggregate> LIMIT k [OFFSET m]`. "Which k
//!   groups score highest (or lowest)?" Position is the answer, so the
//!   result carries a `startingRank`.
//! - **having-range** — `HAVING <aggregate> <op> <value> LIMIT k`.
//!   "Which groups' aggregate falls in this range?" Value is the answer,
//!   so there is no rank and no offset.
//!
//! The grammar for both is not restated here. [`detect_ranked_mode`] and
//! [`detect_having_mode`] are rs-drive's own versioned classifiers — the
//! same functions the server's query table and the proof verifier call —
//! and they are `pub` under the `verify` feature this crate already
//! enables. Calling them client-side means a malformed query fails
//! locally with rs-drive's own message and cannot drift from what the
//! network enforces.

use crate::error::WasmSdkError;
use crate::queries::document::{json_to_platform_value, parse_where_clause};
use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::documents::document_query::{DocumentQuery, RankingDirection};
use dash_sdk::platform::{DataContract, Fetch};
use drive::query::drive_document_having_query::mode_detection::detect_having_mode;
use drive::query::drive_document_ranked_query::mode_detection::detect_ranked_mode;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    RankedAxis, RankedEntry, RankedEntryValue, RankedPaginationInputs, SelectFunction,
    SelectProjection, RANKED_AVG_SCALE,
};
use drive_proof_verifier::{DocumentHavingEntries, DocumentRankedEntries};
use js_sys::{Array, Object, Reflect};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::serialization::conversions::platform_value_to_json;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENTS_RANKED_QUERY_TS: &'static str = r#"
/**
 * The per-group aggregate a ranked / having-range query ranks and
 * filters on.
 *
 * `count` is `COUNT(*)` and takes no property — the count axis counts
 * documents per group. `COUNT(<property>)` is not a ranked axis and is
 * rejected. `sum` / `avg` name the covering index's `summable` property.
 *
 * The covering index must opt in with the matching contract keyword
 * (document meta-schema v3, protocol version 14+): `rankedCountable`,
 * `rankedSummable` or `rankedAverageable`. Without it the node refuses
 * the query and names the keyword the contract has to add.
 */
export type DocumentsAggregateSelect =
  | { type: 'count' }
  | { type: 'sum'; property: string }
  | { type: 'avg'; property: string };

/**
 * One equality pin on a compound ranked index's leading property,
 * spelled like a `DocumentsQuery` where clause: `[property, '==', value]`.
 *
 * A compound ranked index keeps one ordered secondary per prefix value,
 * with no global ordering across prefixes — so a ranked read has to name
 * exactly one prefix, and only `==` names a single value tree. That
 * means: one pin per leading index property, each on a distinct
 * property, and none on the `groupBy` property itself. A single-property
 * ranked index takes no pins at all.
 *
 * A `null` value is legal and addresses the subtree the write path
 * creates for an *absent* optional value.
 *
 * `>` / `<` / `between` / `in` / `startsWith` are all rejected: a range
 * cannot pin one prefix, and `in` would need one secondary walk per
 * element.
 */
export type DocumentsIndexPin = [string, '==' | '=', unknown];

/** How a ranked / having-range walk runs along the aggregate axis. */
export type DocumentsRankDirection = 'asc' | 'desc';

/**
 * `SELECT <aggregate> GROUP BY <property> ORDER BY <that aggregate> LIMIT n [OFFSET m]`
 * — the ranked (top-K) surface, protocol version 14+.
 *
 * Answers "which n groups score highest (or lowest) on an aggregate?"
 * with a proof, by reading a pre-sorted per-axis secondary rather than
 * walking every group's value tree.
 *
 * There is deliberately no `orderBy` here: a ranked query takes exactly
 * one ordering clause and it is always "the selected aggregate", so
 * `direction` is the only free choice. There is no `startAt` /
 * `startAfter` either — a document id does not appear anywhere in a
 * keyspace sorted by aggregate, so a cursor is rejected rather than
 * ignored.
 *
 * @example
 * // The three best restaurants by average grade.
 * const page = await sdk.getDocumentsRanked({
 *   dataContractId: RESTAURANTS,
 *   documentTypeName: 'review',
 *   groupBy: 'restaurantId',
 *   aggregate: { type: 'avg', property: 'grade' },
 *   limit: 3,
 * });
 *
 * @example
 * // The 5th-best restaurant: skip the four above it, take one.
 * const fifth = await sdk.getDocumentsRanked({
 *   dataContractId: RESTAURANTS,
 *   documentTypeName: 'review',
 *   groupBy: 'restaurantId',
 *   aggregate: { type: 'avg', property: 'grade' },
 *   limit: 1,
 *   offset: 4,
 * });
 */
export interface DocumentsRankedQuery {
  /** Data contract identifier. */
  dataContractId: IdentifierLike;

  /** Document type name. */
  documentTypeName: string;

  /**
   * The single `GROUP BY` property — the covering ranked index's
   * *trailing* property, whose distinct values are the ranking's group
   * keys. A compound ranked index ranks each prefix separately: pin
   * every leading property through `where`, and group by the trailing
   * one.
   */
  groupBy: string;

  /** Which aggregate the groups are ranked by. */
  aggregate: DocumentsAggregateSelect;

  /**
   * The ranking's `n`. Required, and `1 <= limit <= 100`.
   *
   * This is a hard ceiling, not a clamp: the limit is echoed inside the
   * proof envelope and re-checked when the client reconstructs the page,
   * so an oversized request is rejected rather than truncated.
   */
  limit: number;

  /**
   * `'desc'` walks from the largest aggregate down — the "top n"
   * reading, where entry 0 is the highest-scoring group. `'asc'` is the
   * "bottom n" reading.
   * @default 'desc'
   */
  direction?: DocumentsRankDirection;

  /**
   * How many ranks to skip before the returned page. There is
   * deliberately no ceiling: the skipped region is attested from counted
   * subtree commitments rather than walked, so a deep offset costs what
   * a shallow one does.
   *
   * An offset past the end of the ranking is a legitimate answer rather
   * than an error — the page comes back empty and `startingRank` is the
   * ranking's whole attested population.
   * @default undefined
   */
  offset?: number;

  /**
   * Equality pins on the covering compound index's leading properties.
   * Omit entirely for a single-property ranked index.
   * @default []
   */
  where?: DocumentsIndexPin[];
}

/**
 * The `HAVING` bound: one contiguous range over the selected aggregate.
 *
 * The aggregate is not restated here. The grammar requires a having
 * clause to bound the same aggregate the query selects, so it is derived
 * from `aggregate` and there is no way to write the mismatch the server
 * would reject.
 *
 * `!=` and `in` describe non-contiguous ranges and are not expressible:
 * a having-range query *is* one contiguous slice of one axis secondary.
 *
 * Operand types follow the axis — `count` bounds are non-negative
 * integers, `sum` bounds are integers in `i64` range, `avg` bounds are
 * numbers in the natural (unscaled) domain of the averaged property.
 * Pass a `bigint` for magnitudes past `Number.MAX_SAFE_INTEGER`.
 *
 * A bound that resolves to an empty range (above the axis maximum, or a
 * `between` whose lower bound exceeds its upper) is rejected rather than
 * silently proving an empty page.
 */
export type DocumentsHavingBound =
  | { operator: '==' | '=' | '>' | '>=' | '<' | '<='; value: number | bigint }
  | {
      operator:
        | 'Between'
        | 'between'
        | 'BetweenExcludeBounds'
        | 'BetweenExcludeLeft'
        | 'BetweenExcludeRight';
      value: [number | bigint, number | bigint];
    };

/**
 * `SELECT <aggregate> GROUP BY <property> HAVING <that aggregate> <op> <value> LIMIT n`
 * — the having-range surface, protocol version 14+.
 *
 * Answers "which groups' aggregate falls in this range?", served as a
 * value-bounded range read of the same axis secondary the ranked surface
 * reads. Verification covers completeness: an in-range group the node
 * omitted fails the proof.
 *
 * Pagination caveat — there is no offset and no cursor. Continuing a
 * page means re-issuing with a tightened bound, which advances past
 * *distinct* aggregate values only, so a page cut inside a tie cannot be
 * continued. Size `limit` above the widest tie you expect.
 *
 * @example
 * // Hashtags with more than 100 posts, biggest first.
 * const hot = await sdk.getDocumentsHaving({
 *   dataContractId: SOCIAL,
 *   documentTypeName: 'post',
 *   groupBy: 'hashtag',
 *   aggregate: { type: 'count' },
 *   having: { operator: '>', value: 100 },
 *   direction: 'desc',
 *   limit: 100,
 * });
 */
export interface DocumentsHavingQuery {
  /** Data contract identifier. */
  dataContractId: IdentifierLike;

  /** Document type name. */
  documentTypeName: string;

  /** The single `GROUP BY` property. Same contract as `DocumentsRankedQuery`. */
  groupBy: string;

  /** Which aggregate the bound applies to. */
  aggregate: DocumentsAggregateSelect;

  /** The one bound. Exactly one clause — multi-clause `AND` is rejected. */
  having: DocumentsHavingBound;

  /** Required, `1 <= limit <= 100`. Same hard-ceiling semantics as ranked. */
  limit: number;

  /**
   * Walk direction along the axis inside the bound. Optional here,
   * unlike ranked, where the ordering *is* the query.
   * @default 'asc'
   */
  direction?: DocumentsRankDirection;

  /**
   * Equality pins on the covering compound index's leading properties.
   * @default []
   */
  where?: DocumentsIndexPin[];
}

/** Which axis a returned aggregate value came from. */
export type DocumentsAggregateKind = 'count' | 'sum' | 'avg';

/** One group in a ranked / having-range result. */
export interface DocumentsGroupEntry {
  /**
   * Hex-encoded raw index-key bytes of the group's value — byte for byte
   * the same key `getDocumentsCount` / `getDocumentsSum` /
   * `getDocumentsAverage` use for the same grouping, so results
   * correlate across the surfaces. Always present, even when
   * `groupValue` could not be produced.
   */
  groupKeyHex: string;

  /**
   * The group key decoded back to its typed value using the contract's
   * document type. Identifiers arrive base58-encoded and byte
   * properties base64-encoded, matching the document JSON convention
   * used elsewhere in this SDK.
   *
   * `null` when the index key is empty — how the write path stores an
   * *absent* optional group-by value. `undefined` when the bytes exist
   * but do not decode as the group-by property's type. `groupKeyHex` is
   * always the lossless fallback.
   */
  groupValue: unknown;

  /**
   * The group's aggregate as an exact integer.
   *
   * - `count` — the document count.
   * - `sum` — the running sum, signed.
   * - `avg` — the *fixed-point* average. Divide by `valueScale` on the
   *   enclosing result; never by a hardcoded literal, because the scale
   *   is a build-time constant that has already changed once.
   *
   * A `bigint` because none of the three fit a JS `number` in general
   * and the average's fixed point is 128-bit. On a proved fetch this is
   * exactly the integer the proof commits to — keep it for comparing
   * groups, reproducing a ranking, or storing. On an unproved fetch the
   * average is reconstructed from the wire's `double`, so digits past
   * f64's ~15 significant decimals are noise. Ranking *order* is exact
   * either way.
   */
  value: bigint;

  /**
   * `value` rendered as a `number`, with the average axis already
   * divided by the scale. A display helper: lossy past 2^53 for counts
   * and sums, and for every average. Two groups whose exact aggregates
   * differ can round to the same `number`, so never compare with this.
   */
  valueAsNumber: number;
}

/** One group in a ranked result, pinned to its absolute position. */
export interface DocumentsRankedEntry extends DocumentsGroupEntry {
  /**
   * The group's 0-based absolute rank, `startingRank + index`. This is
   * what makes `limit: 1, offset: 4` mean "the 5th best" rather than
   * "some entry".
   */
  rank: bigint;
}

/** Result of a ranked (top-K) query. */
export interface DocumentsRankedResult {
  /**
   * The 0-based rank of `entries[0]` — the query's offset as actually
   * honoured. On the proved path this is re-derived from the proof's
   * counted subtree commitments rather than trusted from the node.
   *
   * When `entries` is empty this is a proof that the ranking holds
   * exactly this many groups in total.
   */
  startingRank: bigint;

  /** The groups on this page, in ranking order. Do not re-sort. */
  entries: DocumentsRankedEntry[];

  /** Which axis `entry.value` came from; echoes the request's `aggregate.type`. */
  aggregate: DocumentsAggregateKind;

  /** The request's `groupBy` property, echoed so a result is self-describing. */
  groupBy: string;

  /**
   * Fixed-point divisor for `entry.value`: `1n` for `count` and `sum`,
   * and the build's average scale for `avg`. Returned rather than
   * documented precisely so callers never hardcode it.
   * `Number(e.value) / Number(scale)` is `e.valueAsNumber`; use a
   * decimal library on the two bigints when you need better.
   */
  valueScale: bigint;
}

/** Result of a having-range query. */
export interface DocumentsHavingResult {
  /**
   * The matching groups, in axis order along `direction`. Do not
   * re-sort. There is no rank: a having-range read bounds values, it
   * does not count positions.
   */
  entries: DocumentsGroupEntry[];

  /** Which axis `entry.value` came from. */
  aggregate: DocumentsAggregateKind;

  /** The request's `groupBy` property. */
  groupBy: string;

  /** Same contract as `DocumentsRankedResult.valueScale`. */
  valueScale: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentsRankedQuery")]
    pub type DocumentsRankedQueryJs;

    #[wasm_bindgen(typescript_type = "DocumentsHavingQuery")]
    pub type DocumentsHavingQueryJs;
}

/// The aggregate select, as the JS discriminated union arrives.
///
/// A plain struct rather than an internally-tagged serde enum: the JS
/// object crosses through `platform_value::from_value`, and the
/// buffering-based enum representations are the one serde feature whose
/// behaviour there is not worth betting on. TypeScript still gives
/// callers the compile-time narrowing; this validates the same rules at
/// runtime with a message naming the JS field.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AggregateSelectInput {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    property: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HavingBoundInput {
    operator: String,
    value: JsonValue,
}

/// `deny_unknown_fields` on both query inputs is deliberate, and is a
/// departure from [`super::document`]'s permissive `DocumentsQueryInput`.
///
/// The expected mistake is a caller copying a `DocumentsQuery` object
/// into a ranked call and dragging `orderBy` / `startAfter` along.
/// Permissive serde would silently drop them and *still run the query*
/// under the default direction — answering a different question without
/// saying so. An error is the only honest outcome.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentsRankedQueryInput {
    data_contract_id: IdentifierWasm,
    document_type_name: String,
    group_by: String,
    aggregate: AggregateSelectInput,
    limit: u32,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    offset: Option<u32>,
    /// Pin operands are bounded to `i64::MIN..=u64::MAX`, the same bound the
    /// pre-existing `DocumentsQuery` surface carries: they arrive through
    /// serde-wasm-bindgen's `deserialize_any`, whose BigInt branch refuses
    /// anything wider, and `serde_json::Value` could not hold it either.
    ///
    /// That is not currently a reachable limitation. A contract cannot
    /// declare a 128-bit integer property: the schema path
    /// (`DocumentPropertyType::try_from_value_map` →
    /// `find_integer_type_for_subschema_value`) reads `minimum` / `maximum`
    /// as `i64` and tops out at `U64` / `I64`, and the only constructors of
    /// `DocumentPropertyType::{U128, I128}` are the deprecated
    /// `try_from_name` and the random-document-type test generator. With no
    /// 128-bit property there is no 128-bit index property to pin.
    ///
    /// If DPP ever gains a schema route to those widths, widening this is a
    /// change for the whole document-query surface rather than these two
    /// entry points: all ten share the `Vec<serde_json::Value>` shape, and
    /// the fix has to bypass `deserialize_any` for the operand (reading
    /// `where` off the raw object, with the field declared `IgnoredAny` so
    /// `deny_unknown_fields` still holds).
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentsHavingQueryInput {
    data_contract_id: IdentifierWasm,
    document_type_name: String,
    group_by: String,
    aggregate: AggregateSelectInput,
    having: HavingBoundInput,
    limit: u32,
    #[serde(default)]
    direction: Option<String>,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
}

/// Translate the JS aggregate union into rs-drive's `SELECT` projection.
fn select_from_input(aggregate: &AggregateSelectInput) -> Result<SelectProjection, WasmSdkError> {
    match (aggregate.kind.as_str(), aggregate.property.as_deref()) {
        ("count", None) | ("count", Some("")) => Ok(SelectProjection::count_star()),
        ("count", Some(_)) => Err(WasmSdkError::invalid_argument(
            "aggregate { type: 'count' } takes no `property`: the count axis counts documents \
             per group, which is what COUNT(*) means. COUNT(<property>) is not a ranked axis. \
             Drop `property`, or use { type: 'sum' | 'avg', property } to aggregate a value.",
        )),
        ("sum", Some(property)) if !property.is_empty() => Ok(SelectProjection::sum(property)),
        ("avg", Some(property)) if !property.is_empty() => Ok(SelectProjection::avg(property)),
        (kind @ ("sum" | "avg"), _) => Err(WasmSdkError::invalid_argument(format!(
            "aggregate {{ type: '{kind}' }} requires a non-empty `property` naming the covering \
             ranked index's `summable` property"
        ))),
        (other, _) => Err(WasmSdkError::invalid_argument(format!(
            "unknown aggregate type `{other}`; expected 'count', 'sum' or 'avg'"
        ))),
    }
}

/// `'asc'` / `'desc'` into the named direction pair, or `default` when
/// the caller left the knob unset.
fn ranking_direction(
    direction: Option<&str>,
    default: RankingDirection,
) -> Result<RankingDirection, WasmSdkError> {
    match direction {
        None => Ok(default),
        Some("desc") => Ok(RankingDirection::Descending),
        Some("asc") => Ok(RankingDirection::Ascending),
        Some(other) => Err(WasmSdkError::invalid_argument(format!(
            "direction must be 'asc' or 'desc'; got `{other}`"
        ))),
    }
}

/// Having operators, spelled the way `DocumentWhereOperator` spells the
/// same comparisons so a caller who learned `'>='` for `where` writes
/// `'>='` here too.
///
/// `!=` and `in` are absent on purpose rather than mapped and then
/// rejected downstream: neither describes one contiguous range, and a
/// having-range read is exactly one contiguous slice of one axis.
fn having_operator_from_str(operator: &str) -> Result<HavingOperator, WasmSdkError> {
    match operator {
        "==" | "=" => Ok(HavingOperator::Equal),
        ">" => Ok(HavingOperator::GreaterThan),
        ">=" => Ok(HavingOperator::GreaterThanOrEquals),
        "<" => Ok(HavingOperator::LessThan),
        "<=" => Ok(HavingOperator::LessThanOrEquals),
        "Between" | "between" => Ok(HavingOperator::Between),
        "BetweenExcludeBounds" => Ok(HavingOperator::BetweenExcludeBounds),
        "BetweenExcludeLeft" => Ok(HavingOperator::BetweenExcludeLeft),
        "BetweenExcludeRight" => Ok(HavingOperator::BetweenExcludeRight),
        other => Err(WasmSdkError::invalid_argument(format!(
            "unsupported having operator `{other}`; expected one of '==', '>', '>=', '<', '<=', \
             'Between', 'BetweenExcludeBounds', 'BetweenExcludeLeft', 'BetweenExcludeRight'. \
             '!=' and 'in' describe non-contiguous ranges and cannot be served as a having-range \
             read."
        ))),
    }
}

/// True for the operators whose right-hand operand is a `[lower, upper]`
/// pair rather than a scalar.
fn is_between_operator(operator: HavingOperator) -> bool {
    matches!(
        operator,
        HavingOperator::Between
            | HavingOperator::BetweenExcludeBounds
            | HavingOperator::BetweenExcludeLeft
            | HavingOperator::BetweenExcludeRight
    )
}

/// Build the one having clause from the JS bound.
///
/// The clause's aggregate is *derived* from the select, never supplied
/// by the caller: the grammar requires the two to be equal, so letting
/// JS restate it would only create a failure mode with no upside.
fn having_clause_from_input(
    select: &SelectProjection,
    bound: &HavingBoundInput,
) -> Result<HavingClause, WasmSdkError> {
    let function = match select.function {
        SelectFunction::Count => HavingAggregateFunction::Count,
        SelectFunction::Sum => HavingAggregateFunction::Sum,
        SelectFunction::Avg => HavingAggregateFunction::Avg,
        other => {
            return Err(WasmSdkError::invalid_argument(format!(
                "{other:?} is not a having-range aggregate; expected count, sum or avg"
            )))
        }
    };

    let operator = having_operator_from_str(&bound.operator)?;

    // rs-drive rejects a wrong operand shape too, but naming the JS
    // field is a better message than a bounds-translation failure.
    if is_between_operator(operator)
        && !bound
            .value
            .as_array()
            .map(|operands| operands.len() == 2)
            .unwrap_or(false)
    {
        return Err(WasmSdkError::invalid_argument(format!(
            "having operator `{}` needs `value: [lower, upper]` (exactly two operands)",
            bound.operator
        )));
    }

    Ok(HavingClause {
        aggregate: HavingAggregate {
            function,
            field: select.field.clone(),
        },
        operator,
        right: HavingRightOperand::Value(json_to_platform_value(&bound.value)?),
    })
}

/// Which ranked axis a projection reads.
fn axis_from_select(select: &SelectProjection) -> Result<RankedAxis, WasmSdkError> {
    match select.function {
        SelectFunction::Count => Ok(RankedAxis::Count),
        SelectFunction::Sum => Ok(RankedAxis::Sum),
        SelectFunction::Avg => Ok(RankedAxis::Avg),
        other => Err(WasmSdkError::invalid_argument(format!(
            "{other:?} is not a ranked axis; expected count, sum or avg"
        ))),
    }
}

/// The `DocumentsAggregateKind` string for an axis.
fn axis_kind_str(axis: RankedAxis) -> &'static str {
    match axis {
        RankedAxis::Count => "count",
        RankedAxis::Sum => "sum",
        RankedAxis::Avg => "avg",
    }
}

/// Fixed-point divisor for an axis's entry values.
///
/// Read from rs-drive's re-export of grovedb's constant, never written
/// as a literal: the average scale moved by four orders of magnitude
/// before release, and a JS caller dividing by a stale literal would get
/// plausible-looking wrong numbers rather than an error.
fn value_scale(axis: RankedAxis) -> i128 {
    match axis {
        RankedAxis::Avg => RANKED_AVG_SCALE,
        RankedAxis::Count | RankedAxis::Sum => 1,
    }
}

/// The pagination triple both classifiers take, read off a built query.
fn pagination_inputs(query: &DocumentQuery) -> RankedPaginationInputs {
    RankedPaginationInputs {
        // `DocumentQuery::limit` uses `0` as the "unset" sentinel.
        limit: (query.limit != 0).then_some(query.limit),
        offset: query.offset,
        has_start_at: query.start.is_some(),
    }
}

/// Re-run rs-drive's own versioned ranked grammar client-side.
///
/// This is the same function the server's query table and the proof
/// verifier call — not a copy — so it cannot drift from either, and it
/// is versioned by `platform_version`, so an SDK built against one
/// protocol version cannot quietly accept a shape that version rejects.
fn assert_ranked_shape(
    query: &DocumentQuery,
    platform_version: &PlatformVersion,
) -> Result<(), WasmSdkError> {
    detect_ranked_mode(
        &query.select,
        &query.group_by,
        &query.having,
        &query.order_by_clauses,
        &query.where_clauses,
        pagination_inputs(query),
        platform_version,
    )
    .map(|_| ())
    .map_err(|e| {
        WasmSdkError::invalid_argument(format!(
            "not a well-formed ranked query: {e}. A ranked query is \
             {{ groupBy, aggregate, limit }} plus optional {{ direction, offset, where }}; \
             `where` entries must be `==` pins on the covering compound index's leading \
             properties."
        ))
    })
}

/// Having-range counterpart of [`assert_ranked_shape`].
fn assert_having_shape(
    query: &DocumentQuery,
    platform_version: &PlatformVersion,
) -> Result<(), WasmSdkError> {
    detect_having_mode(
        &query.select,
        &query.group_by,
        &query.having,
        &query.order_by_clauses,
        &query.where_clauses,
        pagination_inputs(query),
        platform_version,
    )
    .map(|_| ())
    .map_err(|e| {
        WasmSdkError::invalid_argument(format!(
            "not a well-formed having-range query: {e}. A having-range query is \
             {{ groupBy, aggregate, having, limit }} plus optional {{ direction, where }}; \
             the bound must describe one contiguous range over the selected aggregate."
        ))
    })
}

/// Turn a `where` list into pins on an already-built query.
fn apply_index_pins(
    mut query: DocumentQuery,
    where_clauses: Option<&[JsonValue]>,
) -> Result<DocumentQuery, WasmSdkError> {
    for clause in where_clauses.unwrap_or(&[]) {
        query = query.with_where(parse_where_clause(clause)?);
    }
    Ok(query)
}

/// Everything about a ranked query except fetching the contract.
///
/// Split out from the async parser so the whole surface is testable on
/// the host target — the async wrapper is a contract fetch plus
/// `DocumentQuery::new` and holds no decisions of its own.
fn apply_ranked_shape(
    base: DocumentQuery,
    input: &DocumentsRankedQueryInput,
    platform_version: &PlatformVersion,
) -> Result<DocumentQuery, WasmSdkError> {
    let group_by = input.group_by.as_str();
    if group_by.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "groupBy must name the covering ranked index's trailing property",
        ));
    }

    let select = select_from_input(&input.aggregate)?;
    let direction = ranking_direction(input.direction.as_deref(), RankingDirection::Descending)?;

    // Order matters. `order_by_selected_aggregate` derives the ordered
    // field from the *current* select (the `$count` sentinel for
    // COUNT(*), the field itself for SUM / AVG), so calling
    // `with_select` after it would leave a stale field name and the
    // server would refuse the request. Owning that sequence here is
    // exactly why the JS surface has no `orderBy` and never mentions the
    // sentinel.
    let mut query = base
        .with_select(select)
        .with_group_by(group_by)
        .order_by_selected_aggregate(direction)
        .with_limit(input.limit);

    if let Some(offset) = input.offset {
        query = query.with_offset(offset);
    }

    let query = apply_index_pins(query, input.where_clauses.as_deref())?;

    assert_ranked_shape(&query, platform_version)?;
    Ok(query)
}

/// Having-range counterpart of [`apply_ranked_shape`].
///
/// The ordering clause is emitted only when the caller asked for a
/// direction: the having grammar makes `ORDER BY` optional and defaults
/// to ascending, and emitting a redundant clause would be one more thing
/// that has to agree with the select.
fn apply_having_shape(
    base: DocumentQuery,
    input: &DocumentsHavingQueryInput,
    platform_version: &PlatformVersion,
) -> Result<DocumentQuery, WasmSdkError> {
    let group_by = input.group_by.as_str();
    if group_by.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "groupBy must name the covering ranked index's trailing property",
        ));
    }

    let select = select_from_input(&input.aggregate)?;
    let having = having_clause_from_input(&select, &input.having)?;

    let mut query = base
        .with_select(select)
        .with_group_by(group_by)
        .with_having(vec![having])
        .with_limit(input.limit);

    if input.direction.is_some() {
        let direction = ranking_direction(input.direction.as_deref(), RankingDirection::Ascending)?;
        query = query.order_by_selected_aggregate(direction);
    }

    let query = apply_index_pins(query, input.where_clauses.as_deref())?;

    assert_having_shape(&query, platform_version)?;
    Ok(query)
}

async fn parse_documents_ranked_query(
    sdk: &WasmSdk,
    query: DocumentsRankedQueryJs,
) -> Result<DocumentQuery, WasmSdkError> {
    let input: DocumentsRankedQueryInput =
        deserialize_required_query(query, "Query object is required", "documents ranked query")?;

    let contract = sdk
        .get_or_fetch_contract(input.data_contract_id.into())
        .await?;
    let base = DocumentQuery::new(contract, &input.document_type_name)?;

    apply_ranked_shape(base, &input, sdk.inner_sdk().version())
}

async fn parse_documents_having_query(
    sdk: &WasmSdk,
    query: DocumentsHavingQueryJs,
) -> Result<DocumentQuery, WasmSdkError> {
    let input: DocumentsHavingQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "documents having-range query",
    )?;

    let contract = sdk
        .get_or_fetch_contract(input.data_contract_id.into())
        .await?;
    let base = DocumentQuery::new(contract, &input.document_type_name)?;

    apply_having_shape(base, &input, sdk.inner_sdk().version())
}

/// One entry decoded as far as it can be without touching a JS type, so
/// that every decision in the result path is host-testable.
#[derive(Debug, Clone, PartialEq)]
struct GroupEntryParts {
    /// Hex of the raw index-key bytes. Lossless, always produced.
    key_hex: String,
    /// The key decoded through the document type, when that succeeded.
    decoded: Option<Value>,
    /// Whether the key was empty — the write path's marker for an absent
    /// optional group-by value, which is a different thing from a key
    /// that failed to decode.
    key_absent: bool,
    /// The group's aggregate, straight off the verified page.
    value: RankedEntryValue,
}

/// Decode a verified page's entries.
///
/// Key decoding is best effort by design: an index key that this
/// contract's decoder cannot read must not fail the whole query, because
/// `key_hex` still answers "which group" for anyone holding the same
/// grouping from the count / sum / average surfaces.
fn group_entry_parts(
    entries: &[RankedEntry],
    document_type: DocumentTypeRef,
    group_by: &str,
    platform_version: &PlatformVersion,
) -> Vec<GroupEntryParts> {
    entries
        .iter()
        .map(|entry| {
            let key_absent = entry.key.is_empty();
            let decoded = if key_absent {
                None
            } else {
                document_type
                    .deserialize_value_for_key(group_by, entry.key.as_slice(), platform_version)
                    .ok()
            };

            GroupEntryParts {
                key_hex: hex::encode(&entry.key),
                decoded,
                key_absent,
                value: entry.value,
            }
        })
        .collect()
}

/// `Reflect::set` on a freshly constructed object, which only fails on a
/// frozen target — the same `expect` convention the aggregate map
/// helpers in [`super::document`] use.
fn set_field(target: &Object, key: &str, value: &JsValue) {
    Reflect::set(target, &JsValue::from_str(key), value)
        .unwrap_or_else(|_| panic!("set {key} on fresh Object"));
}

/// The aggregate as an exact `bigint`, whichever axis it came from.
fn entry_value_to_js(value: RankedEntryValue) -> JsValue {
    match value {
        RankedEntryValue::Count(count) => JsValue::from(count),
        RankedEntryValue::Sum(sum) => JsValue::from(sum),
        RankedEntryValue::AvgFixedPoint(avg) => JsValue::from(avg),
    }
}

/// The integer widths that can leave JavaScript's safe-integer range, and
/// therefore have to cross as exact `BigInt`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExactIntegerKey {
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
}

/// How a decoded group key crosses to JS.
///
/// Classification lives here, in one place, so that
/// [`group_value_to_js`] cannot drift from what the host tests assert —
/// the rendering half touches `js_sys` and so is unreachable off-wasm.
// Not `Eq`: `Value`'s float variant keeps it at `PartialEq`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum GroupValueRepr<'a> {
    /// An exact `BigInt`.
    ExactBigInt(ExactIntegerKey),
    /// The document JSON convention: base58 identifiers, base64 bytes,
    /// `number` for the narrower integer types, strings and bools as-is.
    DocumentJson(&'a Value),
}

/// Decide how a decoded group key should cross to JS.
///
/// `u64`, `i64`, `u128` and `i128` are pulled out of the JSON conversion
/// because that conversion targets a JS `number` and *errors* past
/// `Number.MAX_SAFE_INTEGER` (`serialize_u64` / `serialize_i64` under
/// `json_compatible`), so a single large group key would otherwise reject a
/// whole verified page — and `u128` / `i128` fail earlier still, inside
/// `serde_json`. All four are reachable:
/// `DocumentPropertyType::decode_value_for_tree_keys` returns them for the
/// correspondingly typed properties, and a `Date` group key decodes to
/// `Value::U64`.
///
/// A group key is an identity, not an arithmetic operand, so exactness
/// matters more than the convenience of a `number`. The narrower integer
/// types keep the `number` representation the rest of the document JSON
/// surface uses, which means the JS type follows the property's *declared
/// type* rather than the magnitude of any particular value — a `u64`
/// property is always `bigint`, a `u32` property is always `number`.
fn group_value_repr(value: &Value) -> GroupValueRepr<'_> {
    match value {
        Value::U64(inner) => GroupValueRepr::ExactBigInt(ExactIntegerKey::U64(*inner)),
        Value::I64(inner) => GroupValueRepr::ExactBigInt(ExactIntegerKey::I64(*inner)),
        Value::U128(inner) => GroupValueRepr::ExactBigInt(ExactIntegerKey::U128(*inner)),
        Value::I128(inner) => GroupValueRepr::ExactBigInt(ExactIntegerKey::I128(*inner)),
        other => GroupValueRepr::DocumentJson(other),
    }
}

/// Render a wide integer key as an exact `BigInt`.
///
/// Returns `JsValue`, not `Result<JsValue, _>`, and that is the point: "a
/// large group key can never reject the page it belongs to" is the property
/// this whole classification exists to provide, so it is enforced by the
/// signature rather than left to a test to notice. Making any of these arms
/// fallible would not compile without changing this return type.
fn exact_integer_to_js(key: ExactIntegerKey) -> JsValue {
    match key {
        ExactIntegerKey::U64(inner) => JsValue::from(inner),
        ExactIntegerKey::I64(inner) => JsValue::from(inner),
        ExactIntegerKey::U128(inner) => JsValue::from(inner),
        ExactIntegerKey::I128(inner) => JsValue::from(inner),
    }
}

/// A decoded group key as a JS value. See [`group_value_repr`] for why the
/// wide integer types bypass the JSON conversion.
fn group_value_to_js(value: &Value) -> Result<JsValue, WasmSdkError> {
    match group_value_repr(value) {
        GroupValueRepr::ExactBigInt(key) => Ok(exact_integer_to_js(key)),
        GroupValueRepr::DocumentJson(inner) => {
            platform_value_to_json(inner).map_err(WasmSdkError::from)
        }
    }
}

/// Build one `DocumentsGroupEntry`, optionally carrying an absolute rank.
fn group_entry_to_js(parts: &GroupEntryParts, rank: Option<u64>) -> Result<JsValue, WasmSdkError> {
    let entry = Object::new();

    set_field(&entry, "groupKeyHex", &JsValue::from_str(&parts.key_hex));

    let group_value = match (&parts.decoded, parts.key_absent) {
        (Some(value), _) => group_value_to_js(value)?,
        // Empty key: the group-by value was absent, which is a real
        // group with a known meaning, not a decode failure.
        (None, true) => JsValue::NULL,
        (None, false) => JsValue::UNDEFINED,
    };
    set_field(&entry, "groupValue", &group_value);

    set_field(&entry, "value", &entry_value_to_js(parts.value));
    set_field(
        &entry,
        "valueAsNumber",
        &JsValue::from_f64(parts.value.as_f64()),
    );

    if let Some(rank) = rank {
        set_field(&entry, "rank", &JsValue::from(rank));
    }

    Ok(entry.into())
}

/// Assemble a `DocumentsRankedResult`.
fn ranked_result_to_js(
    parts: &[GroupEntryParts],
    starting_rank: u64,
    axis: RankedAxis,
    group_by: &str,
) -> Result<Object, WasmSdkError> {
    let entries = Array::new();
    for (offset, entry) in parts.iter().enumerate() {
        // `saturating_add` rather than `+`: a caller may legitimately ask
        // for an offset near u32::MAX, and a rank that saturates is a
        // better answer than a debug-build panic.
        let rank = starting_rank.saturating_add(offset as u64);
        entries.push(&group_entry_to_js(entry, Some(rank))?);
    }

    let result = Object::new();
    set_field(&result, "startingRank", &JsValue::from(starting_rank));
    set_field(&result, "entries", &entries.into());
    set_field(
        &result,
        "aggregate",
        &JsValue::from_str(axis_kind_str(axis)),
    );
    set_field(&result, "groupBy", &JsValue::from_str(group_by));
    set_field(&result, "valueScale", &JsValue::from(value_scale(axis)));

    Ok(result)
}

/// Assemble a `DocumentsHavingResult`. No rank: a value-bounded page has
/// no rank base to count from.
fn having_result_to_js(
    parts: &[GroupEntryParts],
    axis: RankedAxis,
    group_by: &str,
) -> Result<Object, WasmSdkError> {
    let entries = Array::new();
    for entry in parts {
        entries.push(&group_entry_to_js(entry, None)?);
    }

    let result = Object::new();
    set_field(&result, "entries", &entries.into());
    set_field(
        &result,
        "aggregate",
        &JsValue::from_str(axis_kind_str(axis)),
    );
    set_field(&result, "groupBy", &JsValue::from_str(group_by));
    set_field(&result, "valueScale", &JsValue::from(value_scale(axis)));

    Ok(result)
}

/// What shaping the result needs, read off the query before `fetch`
/// consumes it.
struct ResultContext {
    axis: RankedAxis,
    group_by: String,
    document_type_name: String,
    data_contract: Arc<DataContract>,
}

impl ResultContext {
    fn from_query(query: &DocumentQuery) -> Result<Self, WasmSdkError> {
        Ok(Self {
            axis: axis_from_select(&query.select)?,
            // Shape assertion already established there is exactly one.
            group_by: query
                .group_by
                .first()
                .cloned()
                .ok_or_else(|| WasmSdkError::invalid_argument("groupBy is required"))?,
            document_type_name: query.document_type_name.clone(),
            data_contract: query.data_contract.clone(),
        })
    }

    fn decode(
        &self,
        entries: &[RankedEntry],
        platform_version: &PlatformVersion,
    ) -> Result<Vec<GroupEntryParts>, WasmSdkError> {
        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {e}")))?;

        Ok(group_entry_parts(
            entries,
            document_type,
            &self.group_by,
            platform_version,
        ))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// The fixed-point divisor the `avg` axis sorts by.
    ///
    /// Exposed so a caller who persisted a `DocumentsGroupEntry.value`
    /// can re-render it later without holding on to the result object
    /// that produced it. Never hardcode the number — it is a build-time
    /// constant that has already changed once.
    #[wasm_bindgen(js_name = "rankedAverageScale", unchecked_return_type = "bigint")]
    pub fn ranked_average_scale() -> JsValue {
        JsValue::from(RANKED_AVG_SCALE)
    }

    /// Hard ceiling on a ranked / having-range `limit`. A request above
    /// it is rejected, not truncated.
    #[wasm_bindgen(js_name = "maxRankedLimit")]
    pub fn max_ranked_limit() -> u32 {
        drive::query::MAX_RANKED_LIMIT as u32
    }

    /// Rank groups by an aggregate and return the top (or bottom) `n`.
    ///
    /// `SELECT <aggregate> GROUP BY <property> ORDER BY <that aggregate>
    /// LIMIT n [OFFSET m]`, served from protocol version 14 against a
    /// contract index that declares the matching `rankedCountable` /
    /// `rankedSummable` / `rankedAverageable` keyword. A node on an
    /// earlier protocol version, or a contract whose index does not opt
    /// in, rejects the query and names what is missing.
    ///
    /// Entries come back in ranking order and must not be re-sorted;
    /// `startingRank` plus the entry's position is its absolute rank.
    #[wasm_bindgen(
        js_name = "getDocumentsRanked",
        unchecked_return_type = "DocumentsRankedResult"
    )]
    pub async fn get_documents_ranked(
        &self,
        query: DocumentsRankedQueryJs,
    ) -> Result<Object, WasmSdkError> {
        let query = parse_documents_ranked_query(self, query).await?;
        let context = ResultContext::from_query(&query)?;

        // An empty ranking proves, so `None` is an empty page rather
        // than an absent answer.
        let page = DocumentRankedEntries::fetch(self.as_ref(), query)
            .await?
            .unwrap_or_default();

        let parts = context.decode(&page.entries, self.inner_sdk().version())?;
        ranked_result_to_js(&parts, page.starting_rank, context.axis, &context.group_by)
    }

    /// [`Self::get_documents_ranked`] with the proof and block metadata
    /// the answer was verified against.
    #[wasm_bindgen(
        js_name = "getDocumentsRankedWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DocumentsRankedResult>"
    )]
    pub async fn get_documents_ranked_with_proof_info(
        &self,
        query: DocumentsRankedQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_documents_ranked_query(self, query).await?;
        let context = ResultContext::from_query(&query)?;

        let (page, metadata, proof) =
            DocumentRankedEntries::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;
        let page = page.unwrap_or_default();

        let parts = context.decode(&page.entries, self.inner_sdk().version())?;
        let result =
            ranked_result_to_js(&parts, page.starting_rank, context.axis, &context.group_by)?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            result, metadata, proof,
        ))
    }

    /// Return the groups whose aggregate falls inside a bound.
    ///
    /// `SELECT <aggregate> GROUP BY <property> HAVING <that aggregate>
    /// <op> <value> LIMIT n`, served from protocol version 14 against
    /// the same ranked indexes [`Self::get_documents_ranked`] reads.
    /// Verification covers completeness — an in-range group the node
    /// omitted fails the proof.
    ///
    /// There is no offset and no cursor: continue a page by tightening
    /// the bound, and size `limit` above the widest expected tie, since
    /// a page cut inside a tie cannot be continued.
    #[wasm_bindgen(
        js_name = "getDocumentsHaving",
        unchecked_return_type = "DocumentsHavingResult"
    )]
    pub async fn get_documents_having(
        &self,
        query: DocumentsHavingQueryJs,
    ) -> Result<Object, WasmSdkError> {
        let query = parse_documents_having_query(self, query).await?;
        let context = ResultContext::from_query(&query)?;

        let page = DocumentHavingEntries::fetch(self.as_ref(), query)
            .await?
            .unwrap_or_default();

        let parts = context.decode(&page.entries, self.inner_sdk().version())?;
        having_result_to_js(&parts, context.axis, &context.group_by)
    }

    /// [`Self::get_documents_having`] with the proof and block metadata
    /// the answer was verified against.
    #[wasm_bindgen(
        js_name = "getDocumentsHavingWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DocumentsHavingResult>"
    )]
    pub async fn get_documents_having_with_proof_info(
        &self,
        query: DocumentsHavingQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_documents_having_query(self, query).await?;
        let context = ResultContext::from_query(&query)?;

        let (page, metadata, proof) =
            DocumentHavingEntries::fetch_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;
        let page = page.unwrap_or_default();

        let parts = context.decode(&page.entries, self.inner_sdk().version())?;
        let result = having_result_to_js(&parts, context.axis, &context.group_by)?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            result, metadata, proof,
        ))
    }
}

/// The one property host tests cannot observe: that a wide integer group key
/// really crosses as an exact JavaScript `BigInt`.
///
/// [`exact_integer_to_js`] returning `JsValue` rather than `Result` makes the
/// path infallible, but a signature cannot pin *representation* — swapping the
/// body for `JsValue::from_f64(inner as f64)` would still compile, still be
/// infallible, and still satisfy every host test, while silently rounding
/// group keys past 2^53. These run on the wasm32 target, against the same
/// function the production result path calls, and need no exported test hook.
///
/// Not run by the default `cargo test`; see the runner command in
/// `Cargo.toml`. (`packages/wasm-drive-verify` carries its wasm tests the
/// same way.)
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// The smallest integer that `f64` cannot represent exactly, so the one
    /// that most cheaply distinguishes a `BigInt` from a rounded `number`.
    const FIRST_INEXACT_IN_F64: u64 = (1u64 << 53) + 1;

    fn js_type_of(value: &JsValue) -> String {
        value
            .js_typeof()
            .as_string()
            .expect("typeof always yields a string")
    }

    #[wasm_bindgen_test]
    fn wide_unsigned_keys_render_as_exact_bigints() {
        for key in [FIRST_INEXACT_IN_F64, u64::MAX] {
            let rendered = exact_integer_to_js(ExactIntegerKey::U64(key));

            assert_eq!(js_type_of(&rendered), "bigint");
            assert_eq!(rendered, JsValue::from(key));
            assert_ne!(
                rendered,
                JsValue::from_f64(key as f64),
                "a rounded number must never pass for the exact key {key}"
            );
        }
    }

    #[wasm_bindgen_test]
    fn wide_signed_keys_render_as_exact_bigints() {
        for key in [-(FIRST_INEXACT_IN_F64 as i64), i64::MIN, i64::MAX] {
            let rendered = exact_integer_to_js(ExactIntegerKey::I64(key));

            assert_eq!(js_type_of(&rendered), "bigint");
            assert_eq!(rendered, JsValue::from(key));
        }
    }

    /// The two widths that cannot even be expressed as a JS `number`, and
    /// which failed inside `serde_json` before this path existed.
    #[wasm_bindgen_test]
    fn oversized_keys_render_as_exact_bigints() {
        let unsigned = exact_integer_to_js(ExactIntegerKey::U128(u128::MAX));
        assert_eq!(js_type_of(&unsigned), "bigint");
        assert_eq!(unsigned, JsValue::from(u128::MAX));

        let signed = exact_integer_to_js(ExactIntegerKey::I128(i128::MIN));
        assert_eq!(js_type_of(&signed), "bigint");
        assert_eq!(signed, JsValue::from(i128::MIN));
    }

    /// The exact integer a proof committed to must survive the whole entry
    /// conversion, not just the renderer in isolation.
    #[wasm_bindgen_test]
    fn a_wide_group_key_survives_the_entry_conversion() {
        let parts = GroupEntryParts {
            key_hex: "ff".to_string(),
            decoded: Some(Value::U64(u64::MAX)),
            key_absent: false,
            value: RankedEntryValue::Count(1),
        };

        let entry =
            group_entry_to_js(&parts, Some(0)).expect("a wide key must not reject the page");
        let group_value = js_sys::Reflect::get(&entry, &JsValue::from_str("groupValue"))
            .expect("the entry carries a groupValue");

        assert_eq!(js_type_of(&group_value), "bigint");
        assert_eq!(group_value, JsValue::from(u64::MAX));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::prelude::DataContract;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use drive::query::{MAX_HAVING_LIMIT, MAX_RANKED_LIMIT, RANKED_COUNT_ORDER_KEY};
    use serde_json::json;

    /// DPNS stands in for any contract here. Every function under test is
    /// shape-only — `detect_ranked_mode` / `detect_having_mode` read no
    /// indexes — so the fixture only has to supply a real document type
    /// with real property names.
    const DOC_TYPE: &str = "domain";
    const GROUP_BY: &str = "normalizedLabel";

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn contract() -> DataContract {
        load_system_data_contract(SystemDataContract::DPNS, platform_version())
            .expect("DPNS contract fixture should load")
    }

    fn base_query() -> DocumentQuery {
        DocumentQuery::new(contract(), DOC_TYPE).expect("DPNS declares a `domain` document type")
    }

    fn count() -> AggregateSelectInput {
        AggregateSelectInput {
            kind: "count".to_string(),
            property: None,
        }
    }

    fn avg(property: &str) -> AggregateSelectInput {
        AggregateSelectInput {
            kind: "avg".to_string(),
            property: Some(property.to_string()),
        }
    }

    fn bound(operator: &str, value: JsonValue) -> HavingBoundInput {
        HavingBoundInput {
            operator: operator.to_string(),
            value,
        }
    }

    /// The contract id is irrelevant to every shape rule under test —
    /// `base_query()` already carries the real fixture contract.
    fn any_contract_id() -> IdentifierWasm {
        IdentifierWasm::from([1u8; 32])
    }

    fn ranked_input(
        group_by: &str,
        aggregate: AggregateSelectInput,
        limit: u32,
        direction: Option<&str>,
        offset: Option<u32>,
        where_clauses: Option<Vec<JsonValue>>,
    ) -> DocumentsRankedQueryInput {
        DocumentsRankedQueryInput {
            data_contract_id: any_contract_id(),
            document_type_name: DOC_TYPE.to_string(),
            group_by: group_by.to_string(),
            aggregate,
            limit,
            direction: direction.map(str::to_string),
            offset,
            where_clauses,
        }
    }

    fn having_input(
        aggregate: AggregateSelectInput,
        bound: HavingBoundInput,
        limit: u32,
        direction: Option<&str>,
    ) -> DocumentsHavingQueryInput {
        DocumentsHavingQueryInput {
            data_contract_id: any_contract_id(),
            document_type_name: DOC_TYPE.to_string(),
            group_by: GROUP_BY.to_string(),
            aggregate,
            having: bound,
            limit,
            direction: direction.map(str::to_string),
            where_clauses: None,
        }
    }

    fn build_ranked(input: DocumentsRankedQueryInput) -> Result<DocumentQuery, WasmSdkError> {
        apply_ranked_shape(base_query(), &input, platform_version())
    }

    fn build_having(input: DocumentsHavingQueryInput) -> Result<DocumentQuery, WasmSdkError> {
        apply_having_shape(base_query(), &input, platform_version())
    }

    /// A ranked query with everything optional left unset.
    fn ranked(aggregate: AggregateSelectInput, limit: u32) -> Result<DocumentQuery, WasmSdkError> {
        build_ranked(ranked_input(GROUP_BY, aggregate, limit, None, None, None))
    }

    /// A having-range query with everything optional left unset.
    fn having(
        aggregate: AggregateSelectInput,
        bound: HavingBoundInput,
        limit: u32,
    ) -> Result<DocumentQuery, WasmSdkError> {
        build_having(having_input(aggregate, bound, limit, None))
    }

    // ---- ranked builder shape ------------------------------------------

    /// The regression test for the ordering trap: because
    /// `order_by_selected_aggregate` reads the *current* select, the
    /// builder has to apply the select first or the ordered field name is
    /// stale. A JS caller cannot get this wrong because they never spell
    /// the ordering at all — this pins that the builder holds the sequence.
    #[test]
    fn ranked_avg_orders_by_the_aggregate_field() {
        let query = ranked(avg("grade"), 3).expect("a well-formed avg ranking");

        assert_eq!(query.order_by_clauses.len(), 1);
        assert_eq!(query.order_by_clauses[0].field, "grade");
        assert!(!query.order_by_clauses[0].ascending);
    }

    /// `COUNT(*)` has no field to order by, so the ranking orders by a
    /// reserved sentinel. The sentinel is deliberately absent from the JS
    /// surface, so this asserts against rs-drive's constant rather than a
    /// literal.
    #[test]
    fn ranked_count_orders_by_the_count_sentinel() {
        let query = ranked(count(), 3).expect("a well-formed count ranking");

        assert_eq!(query.order_by_clauses.len(), 1);
        assert_eq!(query.order_by_clauses[0].field, RANKED_COUNT_ORDER_KEY);
    }

    #[test]
    fn ranked_direction_defaults_to_descending() {
        let query = ranked(count(), 3).expect("a well-formed count ranking");
        assert!(!query.order_by_clauses[0].ascending, "default is top-n");
    }

    #[test]
    fn ranked_direction_asc_is_the_bottom_n_reading() {
        let query = build_ranked(ranked_input(GROUP_BY, count(), 3, Some("asc"), None, None))
            .expect("ascending is a legitimate ranking");

        assert!(query.order_by_clauses[0].ascending);
    }

    #[test]
    fn ranked_direction_garbage_is_rejected() {
        let error = build_ranked(ranked_input(
            GROUP_BY,
            count(),
            3,
            Some("descending"),
            None,
            None,
        ))
        .expect_err("only 'asc' and 'desc' are directions");

        assert!(error.to_string().contains("'asc' or 'desc'"));
    }

    #[test]
    fn ranked_offset_is_carried_and_defaults_to_unset() {
        let fifth_best = build_ranked(ranked_input(GROUP_BY, avg("grade"), 1, None, Some(4), None))
            .expect("limit 1 offset 4 is the fifth-best group");

        assert_eq!(fifth_best.offset, Some(4));
        assert_eq!(fifth_best.limit, 1);

        assert_eq!(ranked(count(), 3).expect("no offset").offset, None);
    }

    /// The limit ceiling is a hard reject rather than a clamp, and the
    /// bound belongs to rs-drive — writing `100` here would let the two
    /// drift apart silently.
    #[test]
    fn ranked_limit_is_bounded_by_max_ranked_limit() {
        assert!(
            ranked(count(), 0).is_err(),
            "a ranking with no limit has no k"
        );
        assert!(
            ranked(count(), MAX_RANKED_LIMIT as u32).is_ok(),
            "the ceiling itself is accepted"
        );
        assert!(
            ranked(count(), MAX_RANKED_LIMIT as u32 + 1).is_err(),
            "past the ceiling is rejected, not truncated"
        );
    }

    #[test]
    fn ranked_count_with_a_property_is_rejected() {
        let aggregate = AggregateSelectInput {
            kind: "count".to_string(),
            property: Some("grade".to_string()),
        };
        let error = ranked(aggregate, 3).expect_err("COUNT(field) is not a ranked axis");

        assert!(error.to_string().contains("COUNT(*)"));
    }

    #[test]
    fn ranked_sum_without_a_property_is_rejected() {
        let aggregate = AggregateSelectInput {
            kind: "sum".to_string(),
            property: None,
        };
        let error = ranked(aggregate, 3).expect_err("SUM() has no field to sum");

        assert!(error
            .to_string()
            .contains("requires a non-empty `property`"));
    }

    #[test]
    fn ranked_unknown_aggregate_type_is_rejected() {
        let aggregate = AggregateSelectInput {
            kind: "median".to_string(),
            property: Some("grade".to_string()),
        };
        let error = ranked(aggregate, 3).expect_err("there are three axes");

        assert!(error.to_string().contains("unknown aggregate type"));
    }

    #[test]
    fn ranked_empty_group_by_is_rejected() {
        let error = build_ranked(ranked_input("", count(), 3, None, None, None))
            .expect_err("a ranking without a grouping ranks nothing");

        assert!(error.to_string().contains("groupBy"));
    }

    // ---- ranked index pins ---------------------------------------------

    #[test]
    fn ranked_equality_pin_is_carried() {
        let query = build_ranked(ranked_input(
            GROUP_BY,
            count(),
            3,
            None,
            None,
            Some(vec![json!(["normalizedParentDomainName", "==", "dash"])]),
        ))
        .expect("one == pin on a leading property");

        assert_eq!(query.where_clauses.len(), 1);
        assert_eq!(query.where_clauses[0].field, "normalizedParentDomainName");
        assert_eq!(
            query.where_clauses[0].value,
            Value::Text("dash".to_string())
        );
    }

    /// A range cannot name one prefix value tree, so it cannot pin a
    /// ranked read's prefix.
    #[test]
    fn ranked_non_equality_pin_is_rejected() {
        let error = build_ranked(ranked_input(
            GROUP_BY,
            count(),
            3,
            None,
            None,
            Some(vec![json!(["normalizedParentDomainName", ">", "dash"])]),
        ))
        .expect_err("a range cannot pin a prefix");

        assert!(error.to_string().contains("ranked"));
    }

    /// The write path gives an absent optional value its own prefix
    /// subtree, and `null` is how a caller addresses it.
    #[test]
    fn ranked_null_pin_addresses_the_absent_value_prefix() {
        let query = build_ranked(ranked_input(
            GROUP_BY,
            count(),
            3,
            None,
            None,
            Some(vec![json!(["normalizedParentDomainName", "==", null])]),
        ))
        .expect("null is a legitimate pin");

        assert_eq!(query.where_clauses[0].value, Value::Null);
    }

    #[test]
    fn ranked_repeated_pin_is_rejected() {
        let error = build_ranked(ranked_input(
            GROUP_BY,
            count(),
            3,
            None,
            None,
            Some(vec![
                json!(["normalizedParentDomainName", "==", "dash"]),
                json!(["normalizedParentDomainName", "==", "dashpay"]),
            ]),
        ))
        .expect_err("a property cannot be pinned to two values at once");

        assert!(error.to_string().contains("ranked"));
    }

    // ---- having-range builder shape ------------------------------------

    /// The caller never restates the aggregate in the bound, so the
    /// mismatch the server rejects is unwritable. This pins that the
    /// clause really is derived from the select.
    #[test]
    fn having_clause_aggregate_mirrors_the_select() {
        let query =
            having(avg("grade"), bound(">", json!(3)), 10).expect("a well-formed avg bound");

        assert_eq!(query.having.len(), 1);
        assert_eq!(
            query.having[0].aggregate,
            HavingAggregate {
                function: HavingAggregateFunction::Avg,
                field: "grade".to_string(),
            }
        );
        assert_eq!(query.having[0].operator, HavingOperator::GreaterThan);
    }

    #[test]
    fn having_count_bound_carries_an_empty_field() {
        let query = having(count(), bound(">", json!(100)), 10).expect("count bounds are legal");

        assert_eq!(
            query.having[0].aggregate,
            HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            }
        );
    }

    #[test]
    fn having_between_requires_two_operands() {
        let error =
            having(count(), bound("between", json!([1])), 10).expect_err("a range needs both ends");
        assert!(error.to_string().contains("[lower, upper]"));

        let query = having(count(), bound("between", json!([1, 2])), 10)
            .expect("two operands is the shape");
        assert_eq!(
            query.having[0].right,
            HavingRightOperand::Value(Value::Array(vec![Value::I64(1), Value::I64(2)]))
        );
    }

    /// Neither describes one contiguous range, and a having-range read is
    /// exactly one contiguous slice of one axis secondary.
    #[test]
    fn having_rejects_non_contiguous_operators() {
        for operator in ["!=", "in", "startsWith"] {
            let error = match having(count(), bound(operator, json!(1)), 10) {
                Ok(_) => panic!("`{operator}` should have been rejected"),
                Err(error) => error.to_string(),
            };

            assert!(
                error.contains("unsupported having operator"),
                "`{operator}` should name the supported set; got: {error}"
            );
        }
    }

    #[test]
    fn having_ordering_is_optional() {
        let unordered = having(count(), bound(">", json!(100)), 10).expect("order by is optional");
        assert!(unordered.order_by_clauses.is_empty());

        let ordered = build_having(having_input(
            count(),
            bound(">", json!(100)),
            10,
            Some("desc"),
        ))
        .expect("a direction sets the walk order");

        assert_eq!(ordered.order_by_clauses.len(), 1);
        assert_eq!(ordered.order_by_clauses[0].field, RANKED_COUNT_ORDER_KEY);
        assert!(!ordered.order_by_clauses[0].ascending);
    }

    #[test]
    fn having_limit_is_bounded_by_max_having_limit() {
        assert!(having(count(), bound(">", json!(1)), 0).is_err());
        assert!(having(count(), bound(">", json!(1)), MAX_HAVING_LIMIT as u32).is_ok());
        assert!(having(count(), bound(">", json!(1)), MAX_HAVING_LIMIT as u32 + 1).is_err());
    }

    // ---- the JS input surface -------------------------------------------

    /// Deserialize a JS-shaped object through the same path
    /// `deserialize_required_query` uses, minus the JS types.
    ///
    /// The success value is discarded: the input structs cannot derive
    /// `Debug` (`IdentifierWasm` does not implement it), and these cases
    /// are all about what the deserializer *refuses*.
    fn ranked_input_from(map: Vec<(&str, Value)>) -> Result<(), String> {
        input_from::<DocumentsRankedQueryInput>(map)
    }

    fn having_input_from(map: Vec<(&str, Value)>) -> Result<(), String> {
        input_from::<DocumentsHavingQueryInput>(map)
    }

    fn input_from<T: serde::de::DeserializeOwned>(map: Vec<(&str, Value)>) -> Result<(), String> {
        let value = Value::Map(
            map.into_iter()
                .map(|(key, value)| (Value::Text(key.to_string()), value))
                .collect(),
        );
        dash_sdk::dpp::platform_value::from_value::<T>(value)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// The expected mistake is pasting a `DocumentsQuery` into a ranked
    /// call. Silently dropping its `orderBy` would answer a different
    /// question under the default direction, so the input refuses it.
    #[test]
    fn ranked_input_rejects_a_pasted_order_by() {
        let error = ranked_input_from(vec![
            ("orderBy", Value::Array(vec![])),
            ("documentTypeName", Value::Text(DOC_TYPE.to_string())),
            ("groupBy", Value::Text(GROUP_BY.to_string())),
            ("limit", Value::U32(3)),
        ])
        .expect_err("orderBy is not part of the ranked surface");

        assert!(error.contains("orderBy"), "got: {error}");
    }

    #[test]
    fn ranked_input_rejects_a_pasted_cursor() {
        for cursor in ["startAt", "startAfter"] {
            let error = ranked_input_from(vec![
                (cursor, Value::Identifier([7u8; 32])),
                ("documentTypeName", Value::Text(DOC_TYPE.to_string())),
                ("groupBy", Value::Text(GROUP_BY.to_string())),
                ("limit", Value::U32(3)),
            ])
            .unwrap_err();

            assert!(error.contains(cursor), "got: {error}");
        }
    }

    /// A having-range page has no rank base, so there is nothing for an
    /// offset to skip from. A caller who expects offset pagination is told
    /// rather than silently served page one.
    #[test]
    fn having_input_rejects_an_offset() {
        let error = having_input_from(vec![
            ("offset", Value::U32(4)),
            ("documentTypeName", Value::Text(DOC_TYPE.to_string())),
            ("groupBy", Value::Text(GROUP_BY.to_string())),
            ("limit", Value::U32(3)),
        ])
        .expect_err("having-range has no offset");

        assert!(error.contains("offset"), "got: {error}");
    }

    // ---- result shaping --------------------------------------------------

    fn parts_for(entries: &[RankedEntry], group_by: &str) -> Vec<GroupEntryParts> {
        let contract = contract();
        let document_type = contract
            .document_type_for_name(DOC_TYPE)
            .expect("DPNS declares a `domain` document type");

        group_entry_parts(entries, document_type, group_by, platform_version())
    }

    /// The hex convention is what lets a ranked result be correlated with
    /// a `getDocumentsCount` map over the same grouping.
    #[test]
    fn group_entry_parts_hex_matches_the_aggregate_map_convention() {
        let entry = RankedEntry {
            key: b"alice".to_vec(),
            value: RankedEntryValue::Count(3),
        };

        let parts = parts_for(&[entry], GROUP_BY);

        assert_eq!(parts[0].key_hex, hex::encode(b"alice"));
        assert!(!parts[0].key_absent);
    }

    #[test]
    fn group_entry_parts_decodes_a_string_group_key() {
        let entry = RankedEntry {
            key: b"alice".to_vec(),
            value: RankedEntryValue::Count(3),
        };

        let parts = parts_for(&[entry], GROUP_BY);

        assert_eq!(parts[0].decoded, Some(Value::Text("alice".to_string())));
    }

    /// An empty key is the write path's marker for an absent optional
    /// group-by value — a real group, distinct from a key that failed to
    /// decode.
    #[test]
    fn group_entry_parts_marks_an_empty_key_absent() {
        let entry = RankedEntry {
            key: Vec::new(),
            value: RankedEntryValue::Count(1),
        };

        let parts = parts_for(&[entry], GROUP_BY);

        assert!(parts[0].key_absent);
        assert_eq!(parts[0].decoded, None);
        assert_eq!(parts[0].key_hex, "");
    }

    /// Decoding is best effort: a key this contract's decoder cannot read
    /// must not fail the whole query, because the hex still identifies the
    /// group.
    #[test]
    fn group_entry_parts_survives_an_undecodable_key() {
        let entry = RankedEntry {
            // `$createdAt` decodes an 8-byte timestamp; two bytes is not one.
            key: vec![0x01, 0x02],
            value: RankedEntryValue::Count(1),
        };

        let parts = parts_for(&[entry], "$createdAt");

        assert_eq!(parts[0].decoded, None);
        assert!(!parts[0].key_absent);
        assert_eq!(parts[0].key_hex, "0102");
    }

    /// The four widths that can leave JavaScript's safe-integer range have
    /// to cross as exact `BigInt`s — the JSON conversion errors on them
    /// rather than rounding, so a single large group key used to reject a
    /// whole verified page.
    ///
    /// Classification is the half that can regress; the rendering half is
    /// infallible by signature ([`exact_integer_to_js`] returns `JsValue`,
    /// not `Result`), so pinning the classification here is what guarantees
    /// a wide key cannot throw. `js_sys` panics off-wasm, which is why the
    /// two halves are split at all.
    #[test]
    fn wide_integer_group_keys_cross_as_exact_bigints() {
        let cases = [
            (Value::U64(u64::MAX), ExactIntegerKey::U64(u64::MAX)),
            (Value::I64(i64::MIN), ExactIntegerKey::I64(i64::MIN)),
            (Value::U128(u128::MAX), ExactIntegerKey::U128(u128::MAX)),
            (Value::I128(i128::MIN), ExactIntegerKey::I128(i128::MIN)),
        ];

        for (value, expected) in cases {
            assert_eq!(
                group_value_repr(&value),
                GroupValueRepr::ExactBigInt(expected),
                "{value:?} must not go through the JSON conversion"
            );
        }
    }

    /// A `Date` group key decodes to `Value::U64`, so timestamps take the
    /// exact path too rather than depending on staying under 2^53.
    #[test]
    fn a_date_group_key_crosses_as_an_exact_bigint() {
        assert_eq!(
            group_value_repr(&Value::U64(1_760_000_000_000)),
            GroupValueRepr::ExactBigInt(ExactIntegerKey::U64(1_760_000_000_000))
        );
    }

    /// Everything narrow enough to be lossless as a JS `number`, and
    /// everything non-numeric, keeps the document JSON convention — so the
    /// JS type follows the property's declared type, not the size of one
    /// value. These variants are unreachable from the WASM-runtime spec,
    /// whose input conversion normalizes every JS number to `i64`.
    #[test]
    fn narrow_and_non_numeric_group_keys_keep_the_json_convention() {
        let cases = [
            Value::U8(7),
            Value::U16(7),
            Value::U32(7),
            Value::I8(-7),
            Value::I16(-7),
            Value::I32(-7),
            Value::Float(1.5),
            Value::Text("alice".to_string()),
            Value::Bool(true),
            Value::Identifier([3u8; 32]),
            Value::Bytes(vec![1, 2, 3]),
            Value::Null,
        ];

        for value in &cases {
            assert!(
                matches!(group_value_repr(value), GroupValueRepr::DocumentJson(_)),
                "{value:?} should keep the document JSON representation"
            );
        }
    }

    /// The average scale is a build-time constant that has already moved
    /// by four orders of magnitude. Anything that hardcodes it produces
    /// plausible-looking wrong numbers rather than an error.
    #[test]
    fn value_scale_reads_the_builds_constant() {
        assert_eq!(value_scale(RankedAxis::Avg), RANKED_AVG_SCALE);
        assert_eq!(value_scale(RankedAxis::Count), 1);
        assert_eq!(value_scale(RankedAxis::Sum), 1);
    }

    #[test]
    fn as_f64_divides_the_avg_variant_by_that_scale() {
        let four = RankedEntryValue::AvgFixedPoint(RANKED_AVG_SCALE * 4);
        assert_eq!(four.as_f64(), 4.0);

        assert_eq!(RankedEntryValue::Count(7).as_f64(), 7.0);
        assert_eq!(RankedEntryValue::Sum(-7).as_f64(), -7.0);
    }

    #[test]
    fn axis_kind_strings_match_the_typescript_union() {
        for (axis, expected) in [
            (RankedAxis::Count, "count"),
            (RankedAxis::Sum, "sum"),
            (RankedAxis::Avg, "avg"),
        ] {
            assert_eq!(axis_kind_str(axis), expected);
            assert!(
                typescript_source().contains(&format!("'{expected}'")),
                "`{expected}` is emitted as an aggregate kind but absent from the \
                 DocumentsAggregateKind union"
            );
        }
    }

    /// `#[wasm_bindgen(typescript_custom_section)]` consumes the const it
    /// is attached to on non-wasm targets, so the declaration text is the
    /// only readable form — and it is the thing under test anyway.
    fn typescript_source() -> String {
        const DECLARATION: &str = "const DOCUMENTS_RANKED_QUERY_TS";
        let source = include_str!("document_ranked.rs");
        let start = source
            .find(DECLARATION)
            .expect("this module declares a TypeScript custom section");
        source[start..].to_string()
    }

    /// The JSDoc states the limit ceiling as a number a caller reads and
    /// relies on. If rs-drive's constant moves, the prose has to move with
    /// it.
    #[test]
    fn typescript_docs_quote_the_real_limit_ceiling() {
        let source = typescript_source();
        let quoted = format!("1 <= limit <= {MAX_RANKED_LIMIT}");

        assert!(
            source.contains(&quoted),
            "the ranked/having JSDoc should state the ceiling as `{quoted}`"
        );
        assert_eq!(
            MAX_RANKED_LIMIT, MAX_HAVING_LIMIT,
            "the docs state one ceiling for both surfaces; split the prose if these diverge"
        );
    }
}
