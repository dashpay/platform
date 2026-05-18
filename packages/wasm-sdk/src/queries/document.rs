use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::Fetch;
use dash_sdk::platform::FetchMany;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::DocumentSplitCounts;
use js_sys::Map;
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
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentsQuery")]
    pub type DocumentsQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentsQueryInput {
    data_contract_id: IdentifierWasm,
    document_type_name: String,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
    #[serde(rename = "orderBy", default)]
    order_by: Option<Vec<JsonValue>>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(rename = "startAfter", default)]
    start_after: Option<IdentifierWasm>,
    #[serde(rename = "startAt", default)]
    start_at: Option<IdentifierWasm>,
    /// Count-query knob: SQL-shaped `GROUP BY` field list,
    /// mirroring the v1 wire `group_by: repeated string` field
    /// one-to-one. Ignored by the regular document-fetch path.
    /// See the TypeScript declaration for the supported shapes.
    /// Default empty (aggregate count).
    #[serde(rename = "groupBy", default)]
    group_by: Option<Vec<String>>,
    // Order direction for count results flows through the existing
    // `orderBy` field — the first clause's direction controls
    // split-mode entry ordering and `(In + prove)` walk order. No
    // separate `orderByAscending` knob.
}

async fn build_documents_query(
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

/// Parse JSON where clause into WhereClause
fn parse_where_clause(json_clause: &JsonValue) -> Result<WhereClause, WasmSdkError> {
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

/// Parse JSON order by clause into OrderClause
fn parse_order_clause(json_clause: &JsonValue) -> Result<OrderClause, WasmSdkError> {
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
fn json_to_platform_value(json_val: &JsonValue) -> Result<Value, WasmSdkError> {
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
    /// **Status**: skeleton — the `DocumentSplitSums::fetch`
    /// `FromProof` impl in `drive-proof-verifier` currently returns
    /// `Error::NotImplemented` until grovedb PR 670 lands the
    /// `verify_aggregate_sum_query` primitive. The wasm wrapper is
    /// here so JS / browser callers can encode against the stable
    /// API surface; calls fail clean with the typed not-implemented
    /// error until then.
    #[wasm_bindgen(
        js_name = "getDocumentsSum",
        unchecked_return_type = "Map<string, bigint>"
    )]
    pub async fn get_documents_sum(
        &self,
        query: DocumentsQueryJs,
        _sum_property: String,
    ) -> Result<Map, WasmSdkError> {
        let _ = query;
        // TODO(sum-feature): mirror `get_documents_count` body —
        // build a `DocumentQuery` via `parse_documents_count_query`
        // (or a parallel `parse_documents_sum_query` that injects
        // `Select::Sum` + `field = sum_property` into the parsed
        // query), then call `DocumentSplitSums::fetch` and map the
        // result via a `split_sums_to_js_map` helper paralleling
        // `split_counts_to_js_map`.
        Err(WasmSdkError::not_implemented("getDocumentsSum"))
    }

    #[wasm_bindgen(
        js_name = "getDocumentsSumWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, bigint>>"
    )]
    pub async fn get_documents_sum_with_proof_info(
        &self,
        query: DocumentsQueryJs,
        _sum_property: String,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let _ = query;
        // TODO(sum-feature): mirror `get_documents_count_with_proof_info`.
        Err(WasmSdkError::not_implemented(
            "getDocumentsSumWithProofInfo",
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
    /// **Status**: skeleton — the `DocumentSplitAverages::fetch`
    /// `FromProof` impl currently returns `Error::NotImplemented`
    /// until grovedb PR 670 lands the
    /// `verify_aggregate_count_and_sum_query` primitive. Same gating
    /// as `getDocumentsSum`.
    #[wasm_bindgen(
        js_name = "getDocumentsAverage",
        unchecked_return_type = "Map<string, {count: bigint, sum: bigint}>"
    )]
    pub async fn get_documents_average(
        &self,
        query: DocumentsQueryJs,
        _sum_property: String,
    ) -> Result<Map, WasmSdkError> {
        let _ = query;
        // TODO(avg-feature): mirror `get_documents_sum` body once it
        // lands. The shape:
        //   1. Build a `DocumentQuery` via a `parse_documents_average_query`
        //      that injects `Select::Avg` + `field = sum_property`.
        //   2. Call `DocumentSplitAverages::fetch`.
        //   3. Map the result via a `split_averages_to_js_map` helper
        //      paralleling `split_sums_to_js_map` — emit
        //      `{count: bigint, sum: bigint}` per entry.
        Err(WasmSdkError::not_implemented("getDocumentsAverage"))
    }

    #[wasm_bindgen(
        js_name = "getDocumentsAverageWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, {count: bigint, sum: bigint}>>"
    )]
    pub async fn get_documents_average_with_proof_info(
        &self,
        query: DocumentsQueryJs,
        _sum_property: String,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let _ = query;
        // TODO(avg-feature): mirror `get_documents_sum_with_proof_info`.
        Err(WasmSdkError::not_implemented(
            "getDocumentsAverageWithProofInfo",
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
