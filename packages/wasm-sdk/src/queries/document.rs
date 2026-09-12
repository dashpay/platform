use crate::queries::utils::deserialize_required_query;
use crate::queries::{ProofInfoWasm, ProofMetadataResponseWasm, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::documents::document_history_query::{
    DocumentHistoryQuery, DocumentHistorySelector,
};
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::Fetch;
use dash_sdk::platform::FetchMany;
use drive::query::{OrderClause, TimeRangeGridSpec, TimeRangeSelector, WhereClause, WhereOperator};
use drive_proof_verifier::types::{DocumentHistory, DocumentHistoryProofInfo};
use drive_proof_verifier::{DocumentSplitAverages, DocumentSplitCounts, DocumentSplitSums};
use js_sys::{BigInt, Map};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENTS_QUERY_TS: &'static str = r#"
/**
 * Supported operators for document query where clauses.
 */
export type DocumentWhereOperator =
  | '=='
  | '='
  | '>'
  | '>='
  | '<'
  | '<='
  | 'Between'
  | 'between'
  | 'BetweenExcludeBounds'
  | 'BetweenExcludeLeft'
  | 'BetweenExcludeRight'
  | 'in'
  | 'In'
  | 'startsWith'
  | 'StartsWith';

/**
 * Document query filtering clause represented as [field, operator, value].
 */
export type DocumentWhereClause = [string, DocumentWhereOperator, unknown];

/**
 * Document ordering clause represented as [field, direction].
 */
export type DocumentOrderByClause = [string, 'asc' | 'desc'];

/**
 * Query parameters for retrieving documents.
 */
export interface DocumentsQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Document type name.
   */
  documentTypeName: string;

  /**
   * Optional filter clauses expressed as [field, operator, value].
   * @default []
   */
  where?: DocumentWhereClause[];

  /**
   * Optional sorting clauses expressed as [field, direction].
   * @default []
   */
  orderBy?: DocumentOrderByClause[];

  /**
   * Maximum number of documents to return.
   * @default 100
   */
  limit?: number;

  /**
   * Exclusive document ID to resume from.
   * @default undefined
   */
  startAfter?: IdentifierLike

  /**
   * Inclusive document ID to start from.
   * @default undefined
   */
  startAt?: IdentifierLike

  /**
   * Count-query knob: SQL-shaped `GROUP BY` field list. Mirrors
   * the v1 wire's `group_by: repeated string` directly. Ignored
   * by the regular document-fetch path.
   *
   * - `[]` or omitted → aggregate count (a single row).
   * - `["<in_field>"]` where `<in_field>` matches an `In`
   *   constraint → per-`In`-value entries (PerInValue).
   * - `["<range_field>"]` where `<range_field>` matches a range
   *   constraint → per-distinct-value entries within the range
   *   (RangeDistinct).
   * - `["<in_field>", "<range_field>"]` for compound `In + range`
   *   queries → compound distinct entries.
   *
   * Entry direction comes from the first `orderBy` clause's
   * direction (which also drives walk order on the materialize +
   * prove path); set `orderBy: [["<range_field>", "asc"|"desc"]]`
   * alongside `groupBy: ["<range_field>"]` to control sort.
   * @default []
   */
  groupBy?: string[];

  /**
   * Time-range bucket selections for "trending"-style queries. Each entry
   * picks a single bucket of a timestamp field covered by a `timeRange`
   * index. For the relative selectors the server resolves the bucket from
   * the current block time and the proof verifier re-derives it from the
   * signed response metadata; `"byStart"` names the bucket absolutely, so
   * both sides read it straight from the query. Provable either way.
   * Requires protocol version 14+ (the first version whose contract
   * grammar hosts `timeRange` indexes).
   *
   * - `selector: "oldest"` → the oldest still-active range (a near-full
   *   trailing window of ~`range`; best for "trending over the last window").
   * - `selector: "newest"` → the freshest started range (latest partial slice).
   * - `selector: "byStart"` → the range starting exactly at `startMs` — any
   *   window, current or historic. `startMs` is then required and must be a
   *   window start on the grid (`phase + k * step`, in milliseconds); an
   *   off-grid start is rejected rather than snapped, and an empty window
   *   is a provable empty answer. The relative selectors must not carry
   *   `startMs`.
   *
   * `grid` names one of the field's grids in the contract's own declared
   * seconds (`{ range, step, phase? }`) — required when the contract buckets
   * the field with more than one `timeRange` grid, where the bare selector
   * is ambiguous and rejected. A zero phase is spelled by omission.
   * @default []
   */
  timeRange?: {
    field: string;
    selector: "newest" | "oldest" | "byStart";
    startMs?: number;
    grid?: { range: number; step: number; phase?: number };
  }[];
}

/**
 * Query parameters for retrieving document history.
 */
export interface DocumentHistoryQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

  /**
   * Document type name.
   */
  documentTypeName: string;

  /**
   * Document identifier.
   */
  documentId: IdentifierLike

  /**
   * Inclusive lower time bound. Supply exactly one selector.
   *
   * Every selector is an exact u64: a `number` is accepted only up to
   * `Number.MAX_SAFE_INTEGER`, and anything larger must be a `bigint`, since
   * JavaScript would have rounded it before the query is built.
   */
  startAtMs?: bigint | number;
  /** Complete exclusive cursor returned by a previous page. */
  startAfter?: { timeMs: bigint | number; revision: bigint | number };
  startAtRevision?: bigint | number;
  revision?: bigint | number;
  /** Maximum ten entries. A single revision requires limit one. */
  limit?: number;
}

export interface DocumentHistoryResult {
  entries: { timeMs: bigint; revision: bigint; document: Document }[];
  lifecycle: {
    /**
     * ACTIVE while the document is visible to ordinary reads, DELETED once it
     * has been deleted and its revisions are retained, ERASING once an
     * authorized erasure has begun, ABSENT when nothing is left.
     */
    state: "ACTIVE" | "DELETED" | "ERASING" | "ABSENT";
    remainingRevisions: bigint;
    /** Zero unless the document has been deleted. */
    deletedAtMs: bigint;
    /** Zero unless an authorized erasure has begun. */
    erasingStartedAtMs: bigint;
    /** Timestamp of the newest revision retained when the erasure began. */
    erasingFromTimeMs: bigint;
    /** History sequence of that revision. */
    erasingFromRevision: bigint;
  };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentsQuery")]
    pub type DocumentsQueryJs;

    #[wasm_bindgen(typescript_type = "DocumentHistoryQuery")]
    pub type DocumentHistoryQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DocumentsQueryInput {
    pub(super) data_contract_id: IdentifierWasm,
    pub(super) document_type_name: String,
    #[serde(rename = "where", default)]
    pub(super) where_clauses: Option<Vec<JsonValue>>,
    #[serde(rename = "orderBy", default)]
    pub(super) order_by: Option<Vec<JsonValue>>,
    #[serde(default)]
    pub(super) limit: Option<u32>,
    #[serde(rename = "startAfter", default)]
    pub(super) start_after: Option<IdentifierWasm>,
    #[serde(rename = "startAt", default)]
    pub(super) start_at: Option<IdentifierWasm>,
    /// Count-query knob: SQL-shaped `GROUP BY` field list,
    /// mirroring the v1 wire `group_by: repeated string` field
    /// one-to-one. Ignored by the regular document-fetch path.
    /// See the TypeScript declaration for the supported shapes.
    /// Default empty (aggregate count).
    #[serde(rename = "groupBy", default)]
    pub(super) group_by: Option<Vec<String>>,
    // Order direction for count results flows through the existing
    // `orderBy` field — the first clause's direction controls
    // split-mode entry ordering and `(In + prove)` walk order. No
    // separate `orderByAscending` knob.
    /// Time-range bucket selections (`IN_TIME_RANGE`), each `{ field,
    /// selector }`. v1-only; resolved server-side from block time.
    #[serde(rename = "timeRange", default)]
    pub(super) time_range: Option<Vec<JsonValue>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentHistoryQueryInput {
    data_contract_id: IdentifierWasm,
    document_type_name: String,
    document_id: IdentifierWasm,
    start_at_ms: Option<ExactU64>,
    start_after: Option<DocumentHistoryCursorInput>,
    start_at_revision: Option<ExactU64>,
    revision: Option<ExactU64>,
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentHistoryCursorInput {
    time_ms: ExactU64,
    revision: ExactU64,
}

/// A `u64` selector that a JavaScript caller may pass as a `bigint` or as a
/// `number`, where a `number` is accepted only while it is a safe integer.
///
/// JavaScript has already rounded any `number` past `Number.MAX_SAFE_INTEGER`
/// by the time it reaches Rust, so silently converting it would query and
/// verify a different timestamp or revision than the caller asked for. A
/// `bigint` carries the exact value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExactU64(u64);

impl<'de> serde::Deserialize<'de> for ExactU64 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = ExactU64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a non-negative integer: a bigint, or a number no larger than Number.MAX_SAFE_INTEGER")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(ExactU64(value))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                u64::try_from(value)
                    .map(ExactU64)
                    .map_err(|_| E::custom("history selectors cannot be negative"))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
                if value.fract() != 0.0 || !(0.0..=MAX_SAFE_INTEGER).contains(&value) {
                    return Err(E::custom(
                        "history selectors must be non-negative integers; pass a bigint for \
                         values past Number.MAX_SAFE_INTEGER",
                    ));
                }
                Ok(ExactU64(value as u64))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl From<ExactU64> for u64 {
    fn from(value: ExactU64) -> Self {
        value.0
    }
}

fn parse_document_history_query(
    query: DocumentHistoryQueryJs,
) -> Result<DocumentHistoryQuery, WasmSdkError> {
    let input: DocumentHistoryQueryInput =
        deserialize_required_query(query, "Query object is required", "document history query")?;
    let selectors = [
        input.start_at_ms.is_some(),
        input.start_after.is_some(),
        input.start_at_revision.is_some(),
        input.revision.is_some(),
    ];
    if selectors.into_iter().filter(|present| *present).count() != 1 {
        return Err(WasmSdkError::invalid_argument(
            "exactly one history selector is required",
        ));
    }
    let selector = if let Some(time) = input.start_at_ms {
        DocumentHistorySelector::StartAtTime(time.into())
    } else if let Some(cursor) = input.start_after {
        DocumentHistorySelector::StartAfter {
            time_ms: cursor.time_ms.into(),
            revision: cursor.revision.into(),
        }
    } else if let Some(revision) = input.start_at_revision {
        DocumentHistorySelector::StartAtRevision(revision.into())
    } else {
        DocumentHistorySelector::Revision(input.revision.expect("one selector was checked").into())
    };
    Ok(DocumentHistoryQuery {
        data_contract_id: input.data_contract_id.into(),
        document_type_name: input.document_type_name,
        document_id: input.document_id.into(),
        selector,
        limit: input.limit,
    })
}

fn document_history_to_js(
    history: DocumentHistory,
    contract_id: Identifier,
    document_type_name: &str,
) -> Result<JsValue, WasmSdkError> {
    use drive_proof_verifier::types::DocumentHistoryState;
    use js_sys::{Array, Object, Reflect};
    let set = |object: &Object, key: &str, value: &JsValue| {
        Reflect::set(object, &JsValue::from_str(key), value).map_err(|_| {
            WasmSdkError::invalid_argument("could not construct document history result")
        })
    };
    let entries = Array::new();
    for entry in history.entries {
        let object = Object::new();
        set(&object, "timeMs", &BigInt::from(entry.time_ms).into())?;
        set(&object, "revision", &BigInt::from(entry.revision).into())?;
        set(
            &object,
            "document",
            &DocumentWasm::new(
                entry.document,
                contract_id,
                document_type_name.to_owned(),
                None,
            )
            .into(),
        )?;
        entries.push(&object);
    }
    let lifecycle = history.lifecycle.ok_or_else(|| {
        WasmSdkError::invalid_argument("history response did not authenticate lifecycle metadata")
    })?;
    let metadata = Object::new();
    set(
        &metadata,
        "state",
        &JsValue::from_str(match lifecycle.state {
            DocumentHistoryState::Active => "ACTIVE",
            DocumentHistoryState::Deleted => "DELETED",
            DocumentHistoryState::Erasing => "ERASING",
            DocumentHistoryState::Absent => "ABSENT",
        }),
    )?;
    set(
        &metadata,
        "remainingRevisions",
        &BigInt::from(lifecycle.remaining_revisions).into(),
    )?;
    // Exact BigInts, like every other timestamp and revision this query
    // returns: a JavaScript number cannot hold a millisecond timestamp
    // without losing precision.
    for (key, value) in [
        ("deletedAtMs", lifecycle.times.deleted_at_ms),
        ("erasingStartedAtMs", lifecycle.times.erasing_started_at_ms),
        ("erasingFromTimeMs", lifecycle.times.erasing_from_time_ms),
        ("erasingFromRevision", lifecycle.times.erasing_from_revision),
    ] {
        set(&metadata, key, &BigInt::from(value).into())?;
    }
    let result = Object::new();
    set(&result, "entries", &entries.into())?;
    set(&result, "lifecycle", &metadata.into())?;
    Ok(result.into())
}

pub(super) async fn build_documents_query(
    sdk: &WasmSdk,
    input: DocumentsQueryInput,
) -> Result<DocumentQuery, WasmSdkError> {
    // `group_by` on the shared input struct is a count-query-only
    // knob; the regular document-fetch path destructured here just
    // drops it.
    let DocumentsQueryInput {
        data_contract_id,
        document_type_name,
        where_clauses,
        order_by,
        limit,
        start_after,
        start_at,
        group_by: _,
        time_range,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    // Fetch contract using cache
    let data_contract = sdk.get_or_fetch_contract(contract_id).await?;

    let mut query = DocumentQuery::new(data_contract, &document_type_name)?;

    query.limit = limit.unwrap_or(100);

    if let Some(start_after_id) = start_after {
        let document_id: Identifier = start_after_id.into();
        query.start = Some(
            dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAfter(
                document_id.to_vec(),
            ),
        );
    } else if let Some(start_at_id) = start_at {
        let document_id: Identifier = start_at_id.into();
        query.start = Some(
            dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAt(
                document_id.to_vec(),
            ),
        );
    }

    if let Some(where_values) = where_clauses {
        for clause_json in where_values.iter() {
            let where_clause = parse_where_clause(clause_json)?;
            query = query.with_where(where_clause);
        }
    }

    if let Some(time_range_values) = time_range {
        for clause_json in time_range_values.iter() {
            let (field, selector, grid) = parse_time_range_clause(clause_json)?;
            query = match grid {
                Some(grid) => query.with_time_range_grid(field, selector, grid),
                None => query.with_time_range(field, selector),
            };
        }
    }

    if let Some(order_values) = order_by {
        for clause_json in order_values.iter() {
            let order_clause = parse_order_clause(clause_json)?;
            query = query.with_order_by(order_clause);
        }
    }

    Ok(query)
}

async fn parse_documents_query(
    sdk: &WasmSdk,
    query: DocumentsQueryJs,
) -> Result<DocumentQuery, WasmSdkError> {
    let input: DocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "documents query")?;

    build_documents_query(sdk, input).await
}

/// Parse a JS query object into a [`DocumentQuery`] configured
/// for the count surface (`select = Count`, with `group_by`
/// taken directly from the input — no implicit translation).
///
/// The JS `groupBy` field mirrors the wire's `group_by: repeated
/// string` one-to-one. Callers ask for exactly the per-group
/// shape they want; the server rejects unsupported
/// `(select, group_by, where)` combinations with
/// `QuerySyntaxError::Unsupported`.
///
/// `orderBy` clauses are consumed by `build_documents_query` and
/// stored on `DocumentQuery.order_by_clauses`, which the SDK
/// request builder serializes into the wire `order_by` field —
/// the first clause's direction controls split-mode entry
/// ordering and is load-bearing for `(In + prove)` walk
/// determinism.
async fn parse_documents_count_query(
    sdk: &WasmSdk,
    query: DocumentsQueryJs,
) -> Result<DocumentQuery, WasmSdkError> {
    let input: DocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "documents count query")?;

    let group_by = input.group_by.clone().unwrap_or_default();
    // DocumentQuery `limit: u32` uses `0` as the "unset" sentinel
    // (translated to `None` on the V1 wire's `optional uint32`).
    // `None` from the JS input maps to that sentinel.
    let limit = input.limit.unwrap_or(0);

    let base_query = build_documents_query(sdk, input).await?;

    Ok(base_query
        .with_select(SelectProjection::count_star())
        .with_group_by_fields(group_by)
        .with_limit(limit))
}

/// Parse a JS query object into a [`DocumentQuery`] configured for
/// the SUM surface (`select = Sum(field)`, with `group_by` taken
/// directly from the input). Sum analog of
/// [`parse_documents_count_query`].
///
/// `sum_property` names the integer document property to aggregate;
/// must match the doctype-level `documentsSummable` OR a per-index
/// `summable: "<prop>"` declaration covering the where-clause shape
/// (the server's index picker enforces this). Empty `sum_property`
/// is rejected here — `SUM()` with no field has no meaning.
async fn parse_documents_sum_query(
    sdk: &WasmSdk,
    query: DocumentsQueryJs,
    sum_property: &str,
) -> Result<DocumentQuery, WasmSdkError> {
    if sum_property.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "sumProperty must be a non-empty string naming the integer document property \
             to sum (matches the doctype's `documentsSummable` or a covering index's \
             `summable: \"<prop>\"`)",
        ));
    }
    let input: DocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "documents sum query")?;

    let group_by = input.group_by.clone().unwrap_or_default();
    let limit = input.limit.unwrap_or(0);

    let base_query = build_documents_query(sdk, input).await?;

    Ok(base_query
        .with_select(SelectProjection::sum(sum_property))
        .with_group_by_fields(group_by)
        .with_limit(limit))
}

/// Parse a JS query object into a [`DocumentQuery`] configured for
/// the AVG surface (`select = Avg(field)`, with `group_by` taken
/// directly from the input). Average analog of
/// [`parse_documents_count_query`].
///
/// The `sum_property` arg names the integer property to average —
/// AVG reuses the sum-tree indexes (no separate `averageable` flag
/// is needed at parse time; the server's picker pairs `summable` +
/// `countable` for the `(count, sum)` shape).
async fn parse_documents_average_query(
    sdk: &WasmSdk,
    query: DocumentsQueryJs,
    sum_property: &str,
) -> Result<DocumentQuery, WasmSdkError> {
    if sum_property.is_empty() {
        return Err(WasmSdkError::invalid_argument(
            "sumProperty must be a non-empty string naming the integer document property \
             to average (matches the doctype's `documentsSummable` / \
             `documentsAverageable`, or a covering index's `summable: \"<prop>\"`)",
        ));
    }
    let input: DocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "documents average query")?;

    let group_by = input.group_by.clone().unwrap_or_default();
    let limit = input.limit.unwrap_or(0);

    let base_query = build_documents_query(sdk, input).await?;

    Ok(base_query
        .with_select(SelectProjection::avg(sum_property))
        .with_group_by_fields(group_by)
        .with_limit(limit))
}

/// Parse JSON where clause into WhereClause
///
/// `pub(super)` so the ranked / having-range surface in
/// [`super::document_ranked`] can reuse the same `[field, operator, value]`
/// spelling a caller already learned here, rather than growing a second
/// where-clause dialect.
pub(super) fn parse_where_clause(json_clause: &JsonValue) -> Result<WhereClause, WasmSdkError> {
    let clause_array = json_clause
        .as_array()
        .ok_or_else(|| WasmSdkError::invalid_argument("where clause must be an array"))?;

    if clause_array.len() != 3 {
        return Err(WasmSdkError::invalid_argument(
            "where clause must have exactly 3 elements: [field, operator, value]",
        ));
    }

    let field = clause_array[0]
        .as_str()
        .ok_or_else(|| WasmSdkError::invalid_argument("where clause field must be a string"))?
        .to_string();

    let operator_str = clause_array[1]
        .as_str()
        .ok_or_else(|| WasmSdkError::invalid_argument("where clause operator must be a string"))?;

    let operator = match operator_str {
        "==" | "=" => WhereOperator::Equal,
        ">" => WhereOperator::GreaterThan,
        ">=" => WhereOperator::GreaterThanOrEquals,
        "<" => WhereOperator::LessThan,
        "<=" => WhereOperator::LessThanOrEquals,
        "Between" | "between" => WhereOperator::Between,
        "BetweenExcludeBounds" => WhereOperator::BetweenExcludeBounds,
        "BetweenExcludeLeft" => WhereOperator::BetweenExcludeLeft,
        "BetweenExcludeRight" => WhereOperator::BetweenExcludeRight,
        "in" | "In" => WhereOperator::In,
        "startsWith" | "StartsWith" => WhereOperator::StartsWith,
        _ => {
            return Err(WasmSdkError::invalid_argument(format!(
                "Unknown operator: {}",
                operator_str
            )));
        }
    };

    // Convert JSON value to platform Value
    let value = json_to_platform_value(&clause_array[2])?;

    Ok(WhereClause {
        field,
        operator,
        value,
    })
}

/// Parse a JSON time-range clause `{ field, selector, startMs?, grid? }`
/// for [`DocumentQuery::with_time_range`] / `with_time_range_grid`.
///
/// `selector` is `"newest"`, `"oldest"`, or `"byStart"`. The relative
/// selectors resolve their window from block time and must not carry
/// `startMs`; `"byStart"` names a window absolutely — `startMs` is then
/// required and must be a window start on the grid (`phase + k * step`,
/// milliseconds), which the server validates against the contract.
///
/// `grid` is `{ range, step, phase? }` in the contract's own declared
/// seconds, naming one of the field's grids — required when the contract
/// buckets the field with more than one `timeRange` grid (the bare selector
/// is ambiguous there and the server rejects it). Like the contract grammar
/// and the wire, a zero phase is spelled by omission.
pub(super) fn parse_time_range_clause(
    json_clause: &JsonValue,
) -> Result<(String, TimeRangeSelector, Option<TimeRangeGridSpec>), WasmSdkError> {
    let object = json_clause.as_object().ok_or_else(|| {
        WasmSdkError::invalid_argument(
            "timeRange clause must be an object { field, selector, startMs?, grid? }",
        )
    })?;
    let field = object
        .get("field")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            WasmSdkError::invalid_argument("timeRange clause requires a string `field`")
        })?
        .to_string();
    let selector_text = object
        .get("selector")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            WasmSdkError::invalid_argument(
                "timeRange clause `selector` must be \"newest\", \"oldest\" or \"byStart\"",
            )
        })?;
    let start_ms = match object.get("startMs") {
        None | Some(JsonValue::Null) => None,
        Some(value) => Some(value.as_u64().ok_or_else(|| {
            WasmSdkError::invalid_argument(
                "timeRange clause `startMs` must be an unsigned integer millisecond timestamp",
            )
        })?),
    };
    let selector = match (selector_text, start_ms) {
        ("byStart", Some(start_ms)) => TimeRangeSelector::ByStart { start_ms },
        ("byStart", None) => {
            return Err(WasmSdkError::invalid_argument(
                "timeRange clause `selector: \"byStart\"` requires `startMs` naming the \
                 window's start (a millisecond timestamp on the grid)",
            ));
        }
        (other, start_ms) => {
            let selector = TimeRangeSelector::from_string(other).ok_or_else(|| {
                WasmSdkError::invalid_argument(
                    "timeRange clause `selector` must be \"newest\", \"oldest\" or \"byStart\"",
                )
            })?;
            if start_ms.is_some() {
                return Err(WasmSdkError::invalid_argument(
                    "timeRange clause `startMs` is only meaningful with `selector: \
                     \"byStart\"`; the relative selectors resolve their window from block \
                     time",
                ));
            }
            selector
        }
    };
    let grid = match object.get("grid") {
        None | Some(JsonValue::Null) => None,
        Some(grid_json) => {
            let grid_object = grid_json.as_object().ok_or_else(|| {
                WasmSdkError::invalid_argument(
                    "timeRange clause `grid` must be an object { range, step, phase? } in \
                     the contract's declared seconds",
                )
            })?;
            let grid_number = |key: &str| -> Result<u64, WasmSdkError> {
                grid_object
                    .get(key)
                    .and_then(JsonValue::as_u64)
                    .ok_or_else(|| {
                        WasmSdkError::invalid_argument(format!(
                            "timeRange clause `grid.{}` must be an unsigned integer of seconds, \
                         exactly as the contract declares it",
                            key
                        ))
                    })
            };
            let range_seconds = grid_number("range")?;
            let step_seconds = grid_number("step")?;
            let phase_seconds = match grid_object.get("phase") {
                None | Some(JsonValue::Null) => 0,
                Some(_) => grid_number("phase")?,
            };
            Some(TimeRangeGridSpec {
                range_seconds,
                step_seconds,
                phase_seconds,
            })
        }
    };
    Ok((field, selector, grid))
}

/// Parse JSON order by clause into OrderClause
pub(super) fn parse_order_clause(json_clause: &JsonValue) -> Result<OrderClause, WasmSdkError> {
    let clause_array = json_clause
        .as_array()
        .ok_or_else(|| WasmSdkError::invalid_argument("order by clause must be an array"))?;

    if clause_array.len() != 2 {
        return Err(WasmSdkError::invalid_argument(
            "order by clause must have exactly 2 elements: [field, direction]",
        ));
    }

    let field = clause_array[0]
        .as_str()
        .ok_or_else(|| WasmSdkError::invalid_argument("order by field must be a string"))?
        .to_string();

    let direction = clause_array[1]
        .as_str()
        .ok_or_else(|| WasmSdkError::invalid_argument("order by direction must be a string"))?;

    let ascending = match direction {
        "asc" => true,
        "desc" => false,
        _ => {
            return Err(WasmSdkError::invalid_argument(
                "order by direction must be 'asc' or 'desc'",
            ));
        }
    };

    Ok(OrderClause { field, ascending })
}

/// Convert JSON value to platform Value
///
/// `pub(super)` for [`super::document_ranked`], which needs the same
/// conversion for a HAVING clause's right-hand operand.
pub(super) fn json_to_platform_value(json_val: &JsonValue) -> Result<Value, WasmSdkError> {
    match json_val {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(b) => Ok(Value::Bool(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::U64(u))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(WasmSdkError::invalid_argument("Unsupported number type"))
            }
        }
        JsonValue::String(s) => Ok(Value::Text(s.clone())),
        JsonValue::Array(arr) => {
            let values: Result<Vec<Value>, WasmSdkError> =
                arr.iter().map(json_to_platform_value).collect();
            Ok(Value::Array(values?))
        }
        JsonValue::Object(obj) => {
            let mut map = Vec::new();
            for (key, val) in obj {
                map.push((Value::Text(key.clone()), json_to_platform_value(val)?));
            }
            Ok(Value::Map(map))
        }
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(
        js_name = "getDocumentHistory",
        unchecked_return_type = "DocumentHistoryResult"
    )]
    pub async fn get_document_history(
        &self,
        query: DocumentHistoryQueryJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = parse_document_history_query(query)?;
        let contract_id = query.data_contract_id;
        let document_type_name = query.document_type_name.clone();
        let contract = self.get_or_fetch_contract(contract_id).await?;
        contract
            .document_type_for_name(&document_type_name)
            .map_err(|error| {
                WasmSdkError::not_found(format!("Document type not found: {error}"))
            })?;
        let history = DocumentHistory::fetch(self.as_ref(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("document history response is missing"))?;
        document_history_to_js(history, contract_id, &document_type_name)
    }

    #[wasm_bindgen(
        js_name = "getDocumentHistoryWithProofInfo",
        unchecked_return_type = "DocumentHistoryProofMetadataResponseTyped"
    )]
    pub async fn get_document_history_with_proof_info(
        &self,
        query: DocumentHistoryQueryJs,
    ) -> Result<DocumentHistoryProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_document_history_query(query)?;
        let contract_id = query.data_contract_id;
        let document_type_name = query.document_type_name.clone();
        let contract = self.get_or_fetch_contract(contract_id).await?;
        contract
            .document_type_for_name(&document_type_name)
            .map_err(|error| {
                WasmSdkError::not_found(format!("Document type not found: {error}"))
            })?;
        let result = DocumentHistoryProofInfo::fetch(self.as_ref(), query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("document history response is missing"))?;
        Ok(DocumentHistoryProofMetadataResponseWasm {
            data: document_history_to_js(result.history, contract_id, &document_type_name)?,
            metadata: result.response.metadata.expect("verified metadata").into(),
            entries_proof: result.response.entries_proof.map(Into::into),
            metadata_proof: result
                .response
                .metadata_proof
                .expect("verified metadata proof")
                .into(),
        })
    }

    #[wasm_bindgen(
        js_name = "getDocuments",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_documents(&self, query: DocumentsQueryJs) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::Documents;

        let query = parse_documents_query(self, query).await?;
        let contract_id = query.data_contract.id();
        let document_type_name = query.document_type_name.clone();

        let documents_result: Documents = Document::fetch_many(self.as_ref(), query).await?;

        let documents_map = Map::new();
        let doc_type_name = document_type_name;

        for (doc_id, doc_opt) in documents_result {
            let key: JsValue = IdentifierWasm::from(doc_id).to_base58().into();

            match doc_opt {
                Some(doc) => {
                    let wasm_doc = DocumentWasm::new(doc, contract_id, doc_type_name.clone(), None);
                    documents_map.set(&key, &JsValue::from(wasm_doc));
                }
                None => {
                    documents_map.set(&key, &JsValue::NULL);
                }
            }
        }

        Ok(documents_map)
    }

    #[wasm_bindgen(
        js_name = "getDocumentsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, Document | undefined>>"
    )]
    pub async fn get_documents_with_proof_info(
        &self,
        query: DocumentsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = parse_documents_query(self, query).await?;
        let contract_id = query.data_contract.id();
        let document_type_name = query.document_type_name.clone();

        let (documents_result, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let documents_map = Map::new();
        let doc_type_name = document_type_name;

        for (doc_id, doc_opt) in documents_result {
            let key: JsValue = IdentifierWasm::from(doc_id).to_base58().into();

            match doc_opt {
                Some(doc) => {
                    let wasm_doc = DocumentWasm::new(doc, contract_id, doc_type_name.clone(), None);
                    documents_map.set(&key, &JsValue::from(wasm_doc));
                }
                None => {
                    documents_map.set(&key, &JsValue::NULL);
                }
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            documents_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(js_name = "getDocument")]
    pub async fn get_document(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")] data_contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "documentType")] document_type: &str,
        #[wasm_bindgen(js_name = "documentId")] document_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        // Parse IDs
        let contract_id: Identifier = data_contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err))
        })?;

        let doc_id: Identifier = document_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid document ID: {}", err))
        })?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Validate document type exists
        data_contract
            .document_type_for_name(document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Create document query using the already-fetched contract
        let query = DocumentQuery::new(data_contract, document_type)?.with_document_id(&doc_id);

        // Execute query
        let document = Document::fetch(self.as_ref(), query)
            .await?
            .map(|doc| DocumentWasm::new(doc, contract_id, document_type.to_string(), None));

        Ok(document)
    }

    #[wasm_bindgen(
        js_name = "getDocumentWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Document | undefined>"
    )]
    pub async fn get_document_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "dataContractId")] data_contract_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "documentType")] document_type: &str,
        #[wasm_bindgen(js_name = "documentId")] document_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse IDs
        let contract_id: Identifier = data_contract_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err))
        })?;

        let doc_id: Identifier = document_id.try_into().map_err(|err| {
            WasmSdkError::invalid_argument(format!("Invalid document ID: {}", err))
        })?;

        // Fetch the data contract (using cache)
        let data_contract = self.get_or_fetch_contract(contract_id).await?;

        // Validate document type exists
        data_contract
            .document_type_for_name(document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Create document query using the already-fetched contract
        let query = DocumentQuery::new(data_contract, document_type)?.with_document_id(&doc_id);

        // Execute query with proof
        let (document_result, metadata, proof) =
            Document::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let document_js = document_result
            .map(|doc| DocumentWasm::new(doc, contract_id, document_type.to_string(), None));

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            JsValue::from(document_js),
            metadata,
            proof,
        ))
    }

    /// Count documents matching a query.
    ///
    /// Returns a `Map<string, bigint>` keyed by the platform-value-
    /// encoded property value (hex-encoded). For simple total counts
    /// (empty / omitted `groupBy`) the map has a single entry with
    /// empty-string key — `result.get("")` is the total. For
    /// per-group modes (non-empty `groupBy`), each key maps to its
    /// count.
    ///
    /// Query-object knobs (all camelCase on the JS side):
    /// - `where: [[field, op, value], ...]`
    /// - `orderBy?: [[field, "asc"|"desc"], ...]` — first clause's
    ///   direction controls per-key entry ordering. On the
    ///   `RangeDistinctProof` prove path the direction is part of
    ///   the path-query bytes the SDK reconstructs to verify the
    ///   proof; empty `orderBy` defaults to ascending on both
    ///   sides. The `PointLookupProof` path (`In` + `prove`, no
    ///   range) doesn't read `orderBy` — its builder sorts In keys
    ///   lex-ascending unconditionally for prove/no-proof parity.
    /// - `limit?: number` — caps the number of entries returned in
    ///   per-group modes. On no-proof paths the server clamps to
    ///   its `max_query_limit`. On the prove-distinct path the
    ///   server rejects oversized requests with `InvalidLimit`
    ///   rather than silently clamping (silent clamping would break
    ///   proof verification); unset falls back to a compile-time
    ///   constant the SDK verifier reads, so proof bytes are
    ///   deterministic across operators regardless of their runtime
    ///   config.
    /// - `groupBy?: string[]` — SQL-shaped GROUP BY, mirroring the
    ///   wire `group_by` field one-to-one. See the `DocumentsQuery`
    ///   TypeScript declaration for the supported shapes (aggregate
    ///   / per-`In`-value / per-distinct-range / compound). The
    ///   server rejects unsupported `(select, group_by, where)`
    ///   combinations with `QuerySyntaxError::Unsupported`.
    ///
    /// One entry point per `[plain | withProofInfo]` variant covers
    /// every count mode because `DocumentSplitCounts::fetch` (which
    /// this wraps) dispatches on the request shape internally. For
    /// compound `In + range` queries with a 2-field `groupBy` the
    /// per-`(in_key, key)` entries are summed by `key` into the flat
    /// map; callers needing the unmerged compound shape should use a
    /// richer binding (not yet exposed here).
    #[wasm_bindgen(
        js_name = "getDocumentsCount",
        unchecked_return_type = "Map<string, bigint>"
    )]
    pub async fn get_documents_count(&self, query: DocumentsQueryJs) -> Result<Map, WasmSdkError> {
        let count_query = parse_documents_count_query(self, query).await?;
        let splits = DocumentSplitCounts::fetch(self.as_ref(), count_query).await?;
        Ok(split_counts_to_js_map(splits))
    }

    #[wasm_bindgen(
        js_name = "getDocumentsCountWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, bigint>>"
    )]
    pub async fn get_documents_count_with_proof_info(
        &self,
        query: DocumentsQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let count_query = parse_documents_count_query(self, query).await?;
        let (splits_opt, metadata, proof) =
            DocumentSplitCounts::fetch_with_metadata_and_proof(self.as_ref(), count_query, None)
                .await?;
        let map = split_counts_to_js_map(splits_opt);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            map, metadata, proof,
        ))
    }

    /// Get aggregated sums of an integer property across documents
    /// matching a query, optionally grouped by an index field.
    ///
    /// Sum-side analog of [`Self::get_documents_count`]. One entry
    /// point per `[plain | withProofInfo]` variant covers every sum
    /// mode (`Aggregate` / `GroupByIn` / `GroupByRange` /
    /// `GroupByCompound`); `DocumentSplitSums::fetch` dispatches
    /// internally on the request shape.
    ///
    /// The map values are `bigint` (signed `i64` on the wire); the
    /// `Aggregate` mode emits a single entry with empty-string key
    /// carrying the total. `GroupByIn` / `GroupByRange` emit one
    /// entry per matched group keyed by the hex-encoded canonical
    /// bytes of the splitting property's value (same convention as
    /// count's per-In / per-distinct-range maps).
    ///
    /// `sumProperty` names the integer document property to
    /// aggregate. Must match the doctype's `documentsSummable` OR a
    /// covering index's `summable: "<prop>"` declaration — the
    /// server's index picker rejects mismatches with a typed
    /// request error.
    #[wasm_bindgen(
        js_name = "getDocumentsSum",
        unchecked_return_type = "Map<string, bigint>"
    )]
    pub async fn get_documents_sum(
        &self,
        query: DocumentsQueryJs,
        sum_property: String,
    ) -> Result<Map, WasmSdkError> {
        let sum_query = parse_documents_sum_query(self, query, &sum_property).await?;
        let splits = DocumentSplitSums::fetch(self.as_ref(), sum_query).await?;
        split_sums_to_js_map(splits)
    }

    #[wasm_bindgen(
        js_name = "getDocumentsSumWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, bigint>>"
    )]
    pub async fn get_documents_sum_with_proof_info(
        &self,
        query: DocumentsQueryJs,
        sum_property: String,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let sum_query = parse_documents_sum_query(self, query, &sum_property).await?;
        let (splits_opt, metadata, proof) =
            DocumentSplitSums::fetch_with_metadata_and_proof(self.as_ref(), sum_query, None)
                .await?;
        let map = split_sums_to_js_map(splits_opt)?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            map, metadata, proof,
        ))
    }

    /// Get the `(count, sum)` pair for the documents matching a query,
    /// optionally grouped by an index field. Client computes
    /// `avg = sum / count`.
    ///
    /// Average-side analog of [`Self::get_documents_sum`]. Returned
    /// map values are `{count: bigint, sum: bigint}` per entry; the
    /// `Aggregate` mode emits a single entry with empty-string key
    /// carrying the totals. JS callers can divide with whichever
    /// representation they want (`Number(sum) / Number(count)`,
    /// BigInt division for integer-truncated, etc.) — the server
    /// intentionally doesn't pre-divide.
    ///
    /// `sumProperty` names the integer document property to
    /// average. AVG reuses the same `documentsSummable` /
    /// `documentsAverageable` index machinery as SUM — no separate
    /// `averageable` flag exists; the server pairs the named
    /// property's `summable` index with a countable terminator to
    /// produce the `(count, sum)` shape.
    #[wasm_bindgen(
        js_name = "getDocumentsAverage",
        unchecked_return_type = "Map<string, {count: bigint, sum: bigint}>"
    )]
    pub async fn get_documents_average(
        &self,
        query: DocumentsQueryJs,
        sum_property: String,
    ) -> Result<Map, WasmSdkError> {
        let avg_query = parse_documents_average_query(self, query, &sum_property).await?;
        let splits = DocumentSplitAverages::fetch(self.as_ref(), avg_query).await?;
        split_averages_to_js_map(splits)
    }

    #[wasm_bindgen(
        js_name = "getDocumentsAverageWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, {count: bigint, sum: bigint}>>"
    )]
    pub async fn get_documents_average_with_proof_info(
        &self,
        query: DocumentsQueryJs,
        sum_property: String,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let avg_query = parse_documents_average_query(self, query, &sum_property).await?;
        let (splits_opt, metadata, proof) =
            DocumentSplitAverages::fetch_with_metadata_and_proof(self.as_ref(), avg_query, None)
                .await?;
        let map = split_averages_to_js_map(splits_opt)?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            map, metadata, proof,
        ))
    }
}

/// Convert an `Option<DocumentSplitCounts>` into a JS `Map<string, bigint>`.
///
/// Keys are hex-encoded so the JS side can match them against the
/// platform-value-encoded property values returned in proofs. None →
/// empty map. For compound (`In + range + distinct`) queries entries
/// carry an `in_key` alongside `key` — to keep this helper's flat-map
/// shape we sum across forks via `into_flat_map`. Callers that need
/// the unmerged per-(in_key, key) view should consume
/// `DocumentSplitCounts.0` directly via a dedicated WASM binding.
fn split_counts_to_js_map(splits: Option<DocumentSplitCounts>) -> Map {
    let map = Map::new();
    if let Some(split_counts) = splits {
        for (key_bytes, count) in split_counts.into_flat_map() {
            let key: JsValue = hex::encode(key_bytes).into();
            map.set(&key, &JsValue::from(count));
        }
    }
    map
}

/// Convert an `Option<DocumentSplitSums>` into a JS `Map<string, bigint>`.
///
/// Sum analog of [`split_counts_to_js_map`]. Same hex-encoded keys,
/// same flat-map fork-merging via
/// `DocumentSplitSums::try_into_flat_map` (which combines
/// per-(in_key, key) entries into per-key sums for compound queries
/// — callers needing the unmerged view should consume
/// `DocumentSplitSums.0` directly).
///
/// Values are `i64` per grovedb's signed SumTree value type.
/// `bigint` on the JS side preserves the full i64 range that
/// `Number` can't — avoids the silent precision loss past
/// `Number.MAX_SAFE_INTEGER` (2^53 - 1) that an `f64` conversion
/// would introduce.
///
/// Returns a `WasmSdkError` if the fold across In-fork branches
/// crosses the i64 range at any terminator key
/// (`try_into_flat_map` does `checked_add` on each step). JS sees
/// a structured error rather than a debug-build panic or a
/// release-build wrap.
fn split_sums_to_js_map(splits: Option<DocumentSplitSums>) -> Result<Map, WasmSdkError> {
    let map = Map::new();
    if let Some(split_sums) = splits {
        // `try_into_flat_map` uses `i64::checked_add` and surfaces
        // overflow as `drive_proof_verifier::Error::RequestError`.
        // Convert to `WasmSdkError::generic` so JS callers see a
        // structured error (rather than the debug-build panic /
        // release-build wrap that the previous unchecked `+=`
        // would produce on a compound-In merge crossing i64::MAX).
        let flat = split_sums
            .try_into_flat_map()
            .map_err(|e| WasmSdkError::generic(format!("{e}")))?;
        for (key_bytes, sum) in flat {
            let key: JsValue = hex::encode(key_bytes).into();
            map.set(&key, &JsValue::from(sum));
        }
    }
    Ok(map)
}

/// Convert an `Option<DocumentSplitAverages>` into a JS `Map<string,
/// {count: bigint, sum: bigint}>`.
///
/// Average analog of [`split_counts_to_js_map`]. Per-entry values
/// are JS objects with `count` (`u64` → `bigint`) and `sum` (`i64`
/// → `bigint`) fields; the JS caller divides with whichever
/// representation it prefers (`Number(sum) / Number(count)` for
/// f64-precision arithmetic, BigInt division for integer-truncated,
/// or its own arbitrary-precision math). The server intentionally
/// doesn't pre-divide — `count` and `sum` are independently
/// load-bearing for downstream filters.
///
/// Hex-encoded keys + `try_into_flat_map` fork-merging match the
/// count and sum helpers' conventions exactly. Returns a
/// `WasmSdkError` if either the u64 count or the i64 sum fold
/// crosses its range at any terminator key, matching the
/// hardening on [`split_sums_to_js_map`].
fn split_averages_to_js_map(splits: Option<DocumentSplitAverages>) -> Result<Map, WasmSdkError> {
    let map = Map::new();
    if let Some(split_averages) = splits {
        // Same overflow hardening rationale as `split_sums_to_js_map`
        // above — `try_into_flat_map` uses `u64::checked_add` (count
        // axis) and `i64::checked_add` (sum axis); either overflow
        // surfaces as a typed JS error instead of a panic / wrap.
        let flat = split_averages
            .try_into_flat_map()
            .map_err(|e| WasmSdkError::generic(format!("{e}")))?;
        for (key_bytes, (count, sum)) in flat {
            let key: JsValue = hex::encode(key_bytes).into();
            let entry = js_sys::Object::new();
            // `unwrap` here is safe in WASM — `js_sys::Reflect::set`
            // only fails on frozen targets, and a freshly-constructed
            // Object is never frozen. Same pattern existing
            // ProofMetadataResponseWasm uses internally.
            js_sys::Reflect::set(&entry, &JsValue::from_str("count"), &JsValue::from(count))
                .expect("set count on fresh Object");
            js_sys::Reflect::set(&entry, &JsValue::from_str("sum"), &JsValue::from(sum))
                .expect("set sum on fresh Object");
            map.set(&key, &entry);
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// This parser is the only boundary turning the public JavaScript
    /// `{ field, selector, startMs?, grid? }` shape into a typed time-range
    /// clause — native and protobuf tests construct their queries after it,
    /// so a parser that dropped the grid or mis-typed a member would not
    /// fail them. Pin every branch here.
    #[test]
    fn parses_a_bare_selector_without_a_grid() {
        let (field, selector, grid) =
            parse_time_range_clause(&json!({ "field": "$createdAt", "selector": "newest" }))
                .expect("the bare shape parses");
        assert_eq!(field, "$createdAt");
        assert_eq!(selector, TimeRangeSelector::Newest);
        assert_eq!(grid, None, "no grid member means the field's sole grid");
    }

    #[test]
    fn parses_a_grid_with_zero_phase_spelled_by_omission() {
        let (_, selector, grid) = parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "oldest",
            "grid": { "range": 21_600, "step": 7_200 },
        }))
        .expect("the grid shape parses");
        assert_eq!(selector, TimeRangeSelector::Oldest);
        assert_eq!(
            grid,
            Some(TimeRangeGridSpec {
                range_seconds: 21_600,
                step_seconds: 7_200,
                phase_seconds: 0,
            }),
            "an omitted (or null) phase is the canonical zero"
        );
    }

    #[test]
    fn parses_a_grid_with_an_explicit_phase() {
        let (_, _, grid) = parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": { "range": 86_400, "step": 86_400, "phase": 3_600 },
        }))
        .expect("the phased grid shape parses");
        assert_eq!(
            grid,
            Some(TimeRangeGridSpec {
                range_seconds: 86_400,
                step_seconds: 86_400,
                phase_seconds: 3_600,
            })
        );
    }

    #[test]
    fn rejects_an_invalid_selector() {
        parse_time_range_clause(&json!({ "field": "$createdAt", "selector": "latest" }))
            .expect_err("only \"newest\", \"oldest\" and \"byStart\" are selectors");
    }

    #[test]
    fn parses_by_start_with_its_window_start() {
        let (field, selector, grid) = parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "byStart",
            "startMs": 1_756_684_800_000u64,
        }))
        .expect("the absolute shape parses");
        assert_eq!(field, "$createdAt");
        assert_eq!(
            selector,
            TimeRangeSelector::ByStart {
                start_ms: 1_756_684_800_000
            }
        );
        assert_eq!(grid, None);
    }

    #[test]
    fn by_start_requires_a_start_and_relative_selectors_refuse_one() {
        parse_time_range_clause(&json!({ "field": "$createdAt", "selector": "byStart" }))
            .expect_err("byStart without startMs names no window");
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "startMs": 1_756_684_800_000u64,
        }))
        .expect_err("a relative selector resolves its window from block time, not startMs");
        // and startMs must be an unsigned integer of milliseconds
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "byStart",
            "startMs": -5,
        }))
        .expect_err("a negative start is not a millisecond timestamp");
    }

    #[test]
    fn rejects_a_missing_field() {
        parse_time_range_clause(&json!({ "selector": "newest" }))
            .expect_err("the field is required");
    }

    #[test]
    fn rejects_a_malformed_grid() {
        // not an object
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": [21_600, 7_200],
        }))
        .expect_err("a grid must be an object");
        // missing step
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": { "range": 21_600 },
        }))
        .expect_err("range and step are both required");
    }

    #[test]
    fn rejects_non_unsigned_grid_members() {
        // negative — the contract's declared units are unsigned seconds
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": { "range": -21_600, "step": 7_200 },
        }))
        .expect_err("a negative range is not a contract-declared value");
        // fractional — sub-second grids don't exist in the grammar
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": { "range": 21_600, "step": 7_200.5 },
        }))
        .expect_err("a fractional step is not a contract-declared value");
        // fractional phase through the optional member
        parse_time_range_clause(&json!({
            "field": "$createdAt",
            "selector": "newest",
            "grid": { "range": 21_600, "step": 7_200, "phase": 0.5 },
        }))
        .expect_err("a fractional phase is not a contract-declared value");
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod history_wasm_tests {
    use super::*;
    use drive_proof_verifier::types::{
        DocumentHistoryEntry, DocumentHistoryLifecycle, DocumentHistoryState,
    };
    use js_sys::{Array, Reflect};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn should_export_both_history_proofs_without_combining_them() {
        let make_proof = |bytes| {
            ProofInfoWasm::from(dash_sdk::platform::proto::Proof {
                grovedb_proof: bytes,
                ..Default::default()
            })
        };
        let result = DocumentHistoryProofMetadataResponseWasm {
            data: JsValue::NULL,
            metadata: dash_sdk::platform::proto::ResponseMetadata::default().into(),
            entries_proof: Some(make_proof(vec![1, 2])),
            metadata_proof: make_proof(vec![3, 4]),
        };
        let result = JsValue::from(result);
        let entries = Reflect::get(&result, &"entriesProof".into()).unwrap();
        let metadata = Reflect::get(&result, &"metadataProof".into()).unwrap();
        assert_eq!(
            js_sys::Uint8Array::new(&Reflect::get(&entries, &"grovedbProof".into()).unwrap())
                .to_vec(),
            vec![1, 2]
        );
        assert_eq!(
            js_sys::Uint8Array::new(&Reflect::get(&metadata, &"grovedbProof".into()).unwrap())
                .to_vec(),
            vec![3, 4]
        );
    }

    #[wasm_bindgen_test]
    fn should_preserve_same_time_revisions_and_exact_lifecycle_counts_in_javascript() {
        let count = (1u64 << 53) + 1;
        let history = DocumentHistory {
            entries: [1, 2]
                .into_iter()
                .map(|revision| DocumentHistoryEntry {
                    time_ms: 2000,
                    revision,
                    document: Document::V0(Default::default()),
                })
                .collect(),
            lifecycle: Some(DocumentHistoryLifecycle {
                state: DocumentHistoryState::Active,
                remaining_revisions: count,
                times: Default::default(),
            }),
        };
        let result = document_history_to_js(history, [1; 32].into(), "note").unwrap();
        let entries = Array::from(&Reflect::get(&result, &"entries".into()).unwrap());
        assert_eq!(entries.length(), 2);
        for (index, revision) in [1u64, 2].into_iter().enumerate() {
            assert_eq!(
                Reflect::get(&entries.get(index as u32), &"revision".into()).unwrap(),
                JsValue::from(revision)
            );
        }
        let lifecycle = Reflect::get(&result, &"lifecycle".into()).unwrap();
        assert_eq!(
            Reflect::get(&lifecycle, &"remainingRevisions".into()).unwrap(),
            JsValue::from(count)
        );
        assert_eq!(
            Reflect::get(&lifecycle, &"state".into()).unwrap(),
            JsValue::from_str("ACTIVE")
        );
    }

    /// Every lifecycle time crosses into JavaScript as an exact BigInt, like
    /// the counts and revisions beside them: a millisecond timestamp does not
    /// survive a JavaScript number.
    #[wasm_bindgen_test]
    fn should_report_the_erasing_state_and_its_times_exactly_in_javascript() {
        use drive::drive::document::history::DocumentHistoryLifecycleTimes;

        let started_at = (1u64 << 53) + 3;
        let history = DocumentHistory {
            entries: vec![],
            lifecycle: Some(DocumentHistoryLifecycle {
                state: DocumentHistoryState::Erasing,
                remaining_revisions: 7,
                times: DocumentHistoryLifecycleTimes {
                    deleted_at_ms: 1_700_000_000_001,
                    erasing_started_at_ms: started_at,
                    erasing_from_time_ms: 1_700_000_000_002,
                    erasing_from_revision: 42,
                },
            }),
        };
        let result = document_history_to_js(history, [1; 32].into(), "note").unwrap();
        let lifecycle = Reflect::get(&result, &"lifecycle".into()).unwrap();
        assert_eq!(
            Reflect::get(&lifecycle, &"state".into()).unwrap(),
            JsValue::from_str("ERASING")
        );
        for (key, expected) in [
            ("deletedAtMs", 1_700_000_000_001u64),
            ("erasingStartedAtMs", started_at),
            ("erasingFromTimeMs", 1_700_000_000_002),
            ("erasingFromRevision", 42),
        ] {
            assert_eq!(
                Reflect::get(&lifecycle, &key.into()).unwrap(),
                JsValue::from(expected),
                "{key} must survive as an exact BigInt"
            );
        }
    }

    #[wasm_bindgen_test]
    fn should_reject_conflicting_selectors_and_public_offsets() {
        let base = r#"{"dataContractId":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec","documentTypeName":"note","documentId":"4mZmxva49PBb7BE7srw9o3gixvDfj1dAx1K6z4A7P9Ah""#;
        for fields in [
            "",
            ",\"startAtMs\":0,\"revision\":1",
            ",\"startAtMs\":0,\"offset\":1",
        ] {
            let input = js_sys::JSON::parse(&format!("{base}{fields}}}")).unwrap();
            assert!(parse_document_history_query(input.unchecked_into()).is_err());
        }
        let input = js_sys::JSON::parse(&format!(
            "{base},\"startAfter\":{{\"timeMs\":2000,\"revision\":22}}}}"
        ))
        .unwrap();
        let query = parse_document_history_query(input.unchecked_into()).unwrap();
        assert_eq!(
            query.selector,
            DocumentHistorySelector::StartAfter {
                time_ms: 2000,
                revision: 22
            }
        );
    }
}

/// History page with the two independently reusable proofs and their shared metadata.
#[wasm_bindgen(js_name = DocumentHistoryProofMetadataResponse)]
pub struct DocumentHistoryProofMetadataResponseWasm {
    data: JsValue,
    metadata: ResponseMetadataWasm,
    entries_proof: Option<ProofInfoWasm>,
    metadata_proof: ProofInfoWasm,
}

#[wasm_bindgen(js_class = DocumentHistoryProofMetadataResponse)]
impl DocumentHistoryProofMetadataResponseWasm {
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> JsValue {
        self.data.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> ResponseMetadataWasm {
        self.metadata.clone()
    }
    #[wasm_bindgen(getter = entriesProof)]
    pub fn entries_proof(&self) -> Option<ProofInfoWasm> {
        self.entries_proof.clone()
    }
    #[wasm_bindgen(getter = metadataProof)]
    pub fn metadata_proof(&self) -> ProofInfoWasm {
        self.metadata_proof.clone()
    }
}

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_HISTORY_PROOF_INFO_TS: &str = r#"
export type DocumentHistoryProofMetadataResponseTyped = DocumentHistoryProofMetadataResponse & { data: DocumentHistoryResult };
"#;

#[cfg(test)]
mod exact_u64_tests {
    use super::ExactU64;

    /// A safe integer passes whether it arrives as an integer or as a whole
    /// floating-point number; anything a JavaScript `number` has already
    /// rounded, and anything fractional or negative, is refused.
    #[test]
    fn should_accept_only_exact_non_negative_integers() {
        assert_eq!(
            serde_json::from_str::<ExactU64>("12").unwrap(),
            ExactU64(12)
        );
        assert_eq!(
            serde_json::from_str::<ExactU64>("9007199254740991").unwrap(),
            ExactU64(9_007_199_254_740_991)
        );
        assert_eq!(
            serde_json::from_str::<ExactU64>("18446744073709551615").unwrap(),
            ExactU64(u64::MAX)
        );
        assert_eq!(
            serde_json::from_str::<ExactU64>("12.0").unwrap(),
            ExactU64(12)
        );
        for rejected in ["12.5", "-1", "-1.0", "9007199254740992.0", "1e300"] {
            assert!(
                serde_json::from_str::<ExactU64>(rejected).is_err(),
                "{rejected} must be refused"
            );
        }
    }
}
