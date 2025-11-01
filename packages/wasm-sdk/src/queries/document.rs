use crate::queries::utils::{deserialize_required_query, identifier_from_base58};
use crate::queries::{ProofInfoWasm, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::{platform_value, Value};
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::Fetch;
use dash_sdk::platform::FetchMany;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use js_sys::Map;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::IdentifierWasm;
#[wasm_bindgen(js_name = "DocumentProofResponse")]
#[derive(Clone)]
pub struct DocumentProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub document: Option<DocumentWasm>,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "DocumentsProofResponse")]
#[derive(Clone)]
pub struct DocumentsProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub documents: Map,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

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
   * Data contract identifier (base58 string).
   */
  dataContractId: string;

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
  startAfter?: string;

  /**
   * Inclusive document ID to start from.
   * @default undefined
   */
  startAt?: string;
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
    data_contract_id: String,
    document_type_name: String,
    #[serde(rename = "where", default)]
    where_clauses: Option<Vec<JsonValue>>,
    #[serde(rename = "orderBy", default)]
    order_by: Option<Vec<JsonValue>>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(rename = "startAfter", default)]
    start_after: Option<String>,
    #[serde(rename = "startAt", default)]
    start_at: Option<String>,
}

async fn build_documents_query(
    sdk: &WasmSdk,
    input: DocumentsQueryInput,
) -> Result<DocumentQuery, WasmSdkError> {
    let DocumentsQueryInput {
        data_contract_id,
        document_type_name,
        where_clauses,
        order_by,
        limit,
        start_after,
        start_at,
    } = input;

    let contract_id = identifier_from_base58(&data_contract_id, "data contract ID")?;

    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, &document_type_name)
            .await?;

    query.limit = limit.unwrap_or(100);

    if let Some(start_after_id) = start_after {
        let document_id = identifier_from_base58(&start_after_id, "startAfter document ID")?;
        query.start = Some(
            dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAfter(
                document_id.to_vec(),
            ),
        );
    } else if let Some(start_at_id) = start_at {
        let document_id = identifier_from_base58(&start_at_id, "startAt document ID")?;
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
            )))
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
            ))
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
        JsonValue::String(s) => {
            // Check if it's an identifier (base58 encoded)
            if s.len() == 44 && s.chars().all(|c| c.is_alphanumeric()) {
                // Try to parse as identifier
                match Identifier::from_string(
                    s,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                ) {
                    Ok(id) => Ok(platform_value!(id)),
                    Err(_) => Ok(Value::Text(s.clone())),
                }
            } else {
                Ok(Value::Text(s.clone()))
            }
        }
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
    #[wasm_bindgen(js_name = "getDocuments")]
    pub async fn get_documents(&self, query: DocumentsQueryJs) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::Documents;

        let query = parse_documents_query(self, query).await?;
        let contract_id = query.data_contract.id().clone();
        let document_type_name = query.document_type_name.clone();

        let documents_result: Documents = Document::fetch_many(self.as_ref(), query).await?;

        let documents_map = Map::new();
        let doc_type_name = document_type_name;

        for (doc_id, doc_opt) in documents_result {
            let key = JsValue::from(IdentifierWasm::from(doc_id));

            match doc_opt {
                Some(doc) => {
                    let wasm_doc =
                        DocumentWasm::from_batch(doc, contract_id, doc_type_name.clone(), None);
                    documents_map.set(&key, &JsValue::from(wasm_doc));
                }
                None => {
                    documents_map.set(&key, &JsValue::NULL);
                }
            }
        }

        Ok(documents_map)
    }

    #[wasm_bindgen(js_name = "getDocumentsWithProofInfo")]
    pub async fn get_documents_with_proof_info(
        &self,
        query: DocumentsQueryJs,
    ) -> Result<DocumentsProofResponseWasm, WasmSdkError> {
        let query = parse_documents_query(self, query).await?;
        let contract_id = query.data_contract.id().clone();
        let document_type_name = query.document_type_name.clone();

        let (documents_result, metadata, proof) =
            Document::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let documents_map = Map::new();
        let doc_type_name = document_type_name;

        for (doc_id, doc_opt) in documents_result {
            let key = JsValue::from(IdentifierWasm::from(doc_id));

            match doc_opt {
                Some(doc) => {
                    let wasm_doc =
                        DocumentWasm::from_batch(doc, contract_id, doc_type_name.clone(), None);
                    documents_map.set(&key, &JsValue::from(wasm_doc));
                }
                None => {
                    documents_map.set(&key, &JsValue::NULL);
                }
            }
        }

        Ok(DocumentsProofResponseWasm {
            documents: documents_map,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getDocument")]
    pub async fn get_document(
        &self,
        data_contract_id: &str,
        document_type: &str,
        document_id: &str,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        use dash_sdk::platform::documents::document_query::DocumentQuery;

        // Parse IDs
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e)))?;

        let doc_id = Identifier::from_string(
            document_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid document ID: {}", e)))?;

        // Create document query
        let query =
            DocumentQuery::new_with_data_contract_id(self.as_ref(), contract_id, document_type)
                .await?
                .with_document_id(&doc_id);

        // Fetch the data contract to get the document type
        let data_contract = dash_sdk::platform::DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        data_contract
            .document_type_for_name(document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Execute query
        let document = Document::fetch(self.as_ref(), query)
            .await?
            .map(|doc| DocumentWasm::from_batch(doc, contract_id, document_type.to_string(), None));

        Ok(document)
    }

    #[wasm_bindgen(js_name = "getDocumentWithProofInfo")]
    pub async fn get_document_with_proof_info(
        &self,
        data_contract_id: &str,
        document_type: &str,
        document_id: &str,
    ) -> Result<DocumentProofResponseWasm, WasmSdkError> {
        use dash_sdk::platform::documents::document_query::DocumentQuery;

        // Parse IDs
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e)))?;

        let doc_id = Identifier::from_string(
            document_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid document ID: {}", e)))?;

        // Create document query
        let query =
            DocumentQuery::new_with_data_contract_id(self.as_ref(), contract_id, document_type)
                .await?
                .with_document_id(&doc_id);

        // Fetch the data contract to get the document type
        let data_contract = dash_sdk::platform::DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        data_contract
            .document_type_for_name(document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Execute query with proof
        let (document_result, metadata, proof) =
            Document::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;

        Ok(DocumentProofResponseWasm {
            document: document_result.map(|doc| {
                DocumentWasm::from_batch(doc, contract_id, document_type.to_string(), None)
            }),
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }
}
