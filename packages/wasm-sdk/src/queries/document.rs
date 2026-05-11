use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::documents::document_count_query::DocumentCountQuery;
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
   * Count-query knob: when `true` AND the query carries a range
   * clause, the server returns per-distinct-value entries within
   * the range instead of a single sum. Ignored by the regular
   * document-fetch path.
   *
   * Entry direction comes from the first `orderBy` clause's
   * direction (which also drives walk order on the materialize +
   * prove path); set `orderBy: [["<range_field>", "asc"|"desc"]]`
   * alongside `returnDistinctCountsInRange: true` to control sort.
   * @default false
   */
  returnDistinctCountsInRange?: boolean;
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
    /// Count-query knob: when `true` AND the query carries a range
    /// clause, the server returns per-distinct-value entries within
    /// the range instead of a single sum. Ignored by the regular
    /// document-fetch path. Default `false`.
    #[serde(default)]
    return_distinct_counts_in_range: Option<bool>,
    // Order direction for count results flows through the existing
    // `orderBy` field — the first clause's direction controls
    // split-mode entry ordering and `(In + prove)` walk order. No
    // separate `orderByAscending` knob.
}

async fn build_documents_query(
    sdk: &WasmSdk,
    input: DocumentsQueryInput,
) -> Result<DocumentQuery, WasmSdkError> {
    // `return_distinct_counts_in_range` on the shared input struct is
    // a count-query-only knob; the regular document-fetch path
    // destructured here just drops it.
    let DocumentsQueryInput {
        data_contract_id,
        document_type_name,
        where_clauses,
        order_by,
        limit,
        start_after,
        start_at,
        return_distinct_counts_in_range: _,
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

/// Parse a JS query object into a [`DocumentCountQuery`] — the count-
/// query analogue of [`parse_documents_query`]. The inner
/// [`DocumentQuery`] is built from the same `DocumentsQueryInput`
/// (data-contract / document-type / where-clauses / orderBy), and the
/// count-specific knobs (`return_distinct_counts_in_range`, `limit`)
/// are forwarded to the outer `DocumentCountQuery` rather than the
/// inner `DocumentQuery`. The SDK-side `TryFrom<&DocumentCountQuery>
/// for DriveDocumentQuery` forcibly nulls the inner limit anyway (so
/// the proof verifier counts every matched doc, not a paginated
/// slice), making the outer-field forwarding load-bearing.
///
/// `orderBy` clauses ARE consumed by `build_documents_query` and
/// stored on `document_query.order_by_clauses`, which the SDK request
/// builder serializes into the wire `order_by` field — the first
/// clause's direction controls split-mode entry ordering and is
/// load-bearing for `(In + prove)` walk determinism.
async fn parse_documents_count_query(
    sdk: &WasmSdk,
    query: DocumentsQueryJs,
) -> Result<DocumentCountQuery, WasmSdkError> {
    let input: DocumentsQueryInput =
        deserialize_required_query(query, "Query object is required", "documents count query")?;

    let return_distinct_counts_in_range = input.return_distinct_counts_in_range.unwrap_or(false);
    let limit = input.limit;

    let base_query = build_documents_query(sdk, input).await?;

    Ok(DocumentCountQuery {
        document_query: base_query,
        return_distinct_counts_in_range,
        limit,
    })
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
    /// (no `in` clause and `return_distinct_counts_in_range = false`)
    /// the map has a single entry with empty-string key —
    /// `result.get("")` is the total. For per-`In`-value or per-
    /// distinct-value-in-range modes, each key maps to its count.
    ///
    /// Query-object knobs (all camelCase on the JS side):
    /// - `where: [[field, op, value], ...]`
    /// - `orderBy?: [[field, "asc"|"desc"], ...]` — first clause's
    ///   direction controls per-key entry ordering. Required when
    ///   the where carries an `In` or range operator on a prove path
    ///   (the materialize-and-count walker needs an explicit order
    ///   for proof determinism).
    /// - `limit?: number` — caps the number of entries returned in
    ///   per-key modes (server clamps to its `max_query_limit`).
    /// - `returnDistinctCountsInRange?: boolean` — when `true` AND
    ///   the query carries a range clause, returns per-distinct-
    ///   value entries instead of a single sum.
    ///
    /// One entry point per `[plain | withProofInfo]` variant covers
    /// every count mode (total / per-`In`-value / per-distinct-value-
    /// in-range / summed-over-range) because `DocumentSplitCounts::
    /// fetch` (which this wraps) dispatches on the request shape
    /// internally. For compound `In + range + distinct` queries the
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
