use crate::sdk::WasmSdk;
use crate::queries::ProofMetadataResponse;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue, JsCast};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::Fetch;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use serde_json::Value as JsonValue;
use drive::query::{WhereClause, WhereOperator, OrderClause};
use dash_sdk::dpp::platform_value::{Value, platform_value};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DocumentResponse {
    id: String,
    owner_id: String,
    revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transferred_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at_block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at_block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transferred_at_block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at_core_block_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at_core_block_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transferred_at_core_block_height: Option<u32>,
    data: serde_json::Map<String, JsonValue>,
}

impl DocumentResponse {
    fn from_document(
        doc: &Document,
        _data_contract: &dash_sdk::platform::DataContract,
        _document_type: dash_sdk::dpp::data_contract::document_type::DocumentTypeRef,
    ) -> Result<Self, JsError> {
        use dash_sdk::dpp::document::DocumentV0Getters;

        // For now, we'll continue with the existing approach
        // In the future, we could use the document type to better interpret the data

        // Get document properties and convert each to JSON
        let mut data = serde_json::Map::new();
        let properties = doc.properties();

        for (key, value) in properties {
            // Convert platform Value to JSON
            let json_value: JsonValue = value
                .clone()
                .try_into()
                .map_err(|e| JsError::new(&format!("Failed to convert value to JSON: {:?}", e)))?;

            data.insert(key.clone(), json_value);
        }

        let response = Self {
            id: doc
                .id()
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            owner_id: doc
                .owner_id()
                .to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            revision: doc.revision().unwrap_or(0),
            created_at: doc.created_at(),
            updated_at: doc.updated_at(),
            transferred_at: doc.transferred_at(),
            created_at_block_height: doc.created_at_block_height(),
            updated_at_block_height: doc.updated_at_block_height(),
            transferred_at_block_height: doc.transferred_at_block_height(),
            created_at_core_block_height: doc.created_at_core_block_height(),
            updated_at_core_block_height: doc.updated_at_core_block_height(),
            transferred_at_core_block_height: doc.transferred_at_core_block_height(),
            data,
        };

        Ok(response)
    }
}

/// Parse JSON where clause into WhereClause
fn parse_where_clause(json_clause: &JsonValue) -> Result<WhereClause, JsError> {
    let clause_array = json_clause
        .as_array()
        .ok_or_else(|| JsError::new("where clause must be an array"))?;

    if clause_array.len() != 3 {
        return Err(JsError::new(
            "where clause must have exactly 3 elements: [field, operator, value]",
        ));
    }

    let field = clause_array[0]
        .as_str()
        .ok_or_else(|| JsError::new("where clause field must be a string"))?
        .to_string();

    let operator_str = clause_array[1]
        .as_str()
        .ok_or_else(|| JsError::new("where clause operator must be a string"))?;

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
        _ => return Err(JsError::new(&format!("Unknown operator: {}", operator_str))),
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
fn parse_order_clause(json_clause: &JsonValue) -> Result<OrderClause, JsError> {
    let clause_array = json_clause
        .as_array()
        .ok_or_else(|| JsError::new("order by clause must be an array"))?;

    if clause_array.len() != 2 {
        return Err(JsError::new(
            "order by clause must have exactly 2 elements: [field, direction]",
        ));
    }

    let field = clause_array[0]
        .as_str()
        .ok_or_else(|| JsError::new("order by field must be a string"))?
        .to_string();

    let direction = clause_array[1]
        .as_str()
        .ok_or_else(|| JsError::new("order by direction must be a string"))?;

    let ascending = match direction {
        "asc" => true,
        "desc" => false,
        _ => return Err(JsError::new("order by direction must be 'asc' or 'desc'")),
    };

    Ok(OrderClause { field, ascending })
}

/// Convert JSON value to platform Value
fn json_to_platform_value(json_val: &JsonValue) -> Result<Value, JsError> {
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
                Err(JsError::new("Unsupported number type"))
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
            let values: Result<Vec<Value>, JsError> =
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
pub async fn get_documents(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type: &str,
    where_clause: Option<String>,
    order_by: Option<String>,
    limit: Option<u32>,
    start_after: Option<String>,
    start_at: Option<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use dash_sdk::platform::FetchMany;
    use drive_proof_verifier::types::Documents;

    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create base document query
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, document_type)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Set limit if provided
    if let Some(limit_val) = limit {
        query.limit = limit_val;
    } else {
        query.limit = 100; // Default limit
    }

    // Handle start parameters
    if let Some(start_after_id) = start_after {
        let doc_id = Identifier::from_string(
            &start_after_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        query.start = Some(dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAfter(
            doc_id.to_vec()
        ));
    } else if let Some(start_at_id) = start_at {
        let doc_id = Identifier::from_string(
            &start_at_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        query.start = Some(dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAt(
            doc_id.to_vec()
        ));
    }

    // Parse and apply where clauses
    if let Some(where_json) = where_clause {
        let json_value: JsonValue = serde_json::from_str(&where_json)
            .map_err(|e| JsError::new(&format!("Failed to parse where clause JSON: {}", e)))?;

        // Expect an array of where clauses
        let where_array = json_value
            .as_array()
            .ok_or_else(|| JsError::new("where clause must be an array of clauses"))?;

        for clause_json in where_array {
            let where_clause = parse_where_clause(clause_json)?;
            query = query.with_where(where_clause);
        }
    }

    // Parse and apply order by clauses
    if let Some(order_json) = order_by {
        let json_value: JsonValue = serde_json::from_str(&order_json)
            .map_err(|e| JsError::new(&format!("Failed to parse order by JSON: {}", e)))?;

        // Expect an array of order clauses
        let order_array = json_value
            .as_array()
            .ok_or_else(|| JsError::new("order by must be an array of clauses"))?;

        for clause_json in order_array {
            let order_clause = parse_order_clause(clause_json)?;
            query = query.with_order_by(order_clause);
        }
    }

    // Execute query
    let documents_result: Documents = Document::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch documents: {}", e)))?;

    // Fetch the data contract to get the document type
    let data_contract = dash_sdk::platform::DataContract::fetch(sdk.as_ref(), contract_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch data contract: {}", e)))?
        .ok_or_else(|| JsError::new("Data contract not found"))?;

    // Get the document type
    let document_type_ref = data_contract
        .document_type_for_name(document_type)
        .map_err(|e| JsError::new(&format!("Document type not found: {}", e)))?;

    // Convert documents to response format
    let mut responses: Vec<DocumentResponse> = Vec::new();
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            responses.push(DocumentResponse::from_document(
                &doc,
                &data_contract,
                document_type_ref,
            )?);
        }
    }

    // Use json_compatible serializer to convert maps to objects
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    responses
        .serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_documents_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type: &str,
    where_clause: Option<String>,
    order_by: Option<String>,
    limit: Option<u32>,
    start_after: Option<String>,
    start_at: Option<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use dash_sdk::platform::FetchMany;

    // Parse data contract ID
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create base document query
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, document_type)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Set limit if provided
    if let Some(limit_val) = limit {
        query.limit = limit_val;
    } else {
        query.limit = 100; // Default limit
    }

    // Handle start parameters
    if let Some(start_after_id) = start_after {
        let doc_id = Identifier::from_string(
            &start_after_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        query.start = Some(dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAfter(
            doc_id.to_vec()
        ));
    } else if let Some(start_at_id) = start_at {
        let doc_id = Identifier::from_string(
            &start_at_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        query.start = Some(dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start::StartAt(
            doc_id.to_vec()
        ));
    }

    // Parse and set where clauses if provided
    if let Some(where_json) = where_clause {
        let clauses: Vec<JsonValue> = serde_json::from_str(&where_json)
            .map_err(|e| JsError::new(&format!("Invalid where clause JSON: {}", e)))?;

        for clause_json in clauses {
            let where_clause = parse_where_clause(&clause_json)?;
            query = query.with_where(where_clause);
        }
    }

    // Parse and set order by clauses if provided
    if let Some(order_json) = order_by {
        let clauses: Vec<JsonValue> = serde_json::from_str(&order_json)
            .map_err(|e| JsError::new(&format!("Invalid order by JSON: {}", e)))?;

        for clause_json in clauses {
            let order_clause = parse_order_clause(&clause_json)?;
            query = query.with_order_by(order_clause);
        }
    }

    // Execute query with proof
    let (documents_result, metadata, proof) =
        Document::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch documents: {}", e)))?;

    // Fetch the data contract to get the document type
    let data_contract = dash_sdk::platform::DataContract::fetch(sdk.as_ref(), contract_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch data contract: {}", e)))?
        .ok_or_else(|| JsError::new("Data contract not found"))?;

    // Get the document type
    let document_type_ref = data_contract
        .document_type_for_name(document_type)
        .map_err(|e| JsError::new(&format!("Document type not found: {}", e)))?;

    // Convert documents to response format
    let mut responses: Vec<DocumentResponse> = Vec::new();
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            responses.push(DocumentResponse::from_document(
                &doc,
                &data_contract,
                document_type_ref,
            )?);
        }
    }

    let response = ProofMetadataResponse {
        data: responses,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_document(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type: &str,
    document_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;

    // Parse IDs
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let doc_id = Identifier::from_string(
        document_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create document query
    let query = DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, document_type)
        .await
        .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);

    // Fetch the data contract to get the document type
    let data_contract = dash_sdk::platform::DataContract::fetch(sdk.as_ref(), contract_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch data contract: {}", e)))?
        .ok_or_else(|| JsError::new("Data contract not found"))?;

    // Get the document type
    let document_type = data_contract
        .document_type_for_name(document_type)
        .map_err(|e| JsError::new(&format!("Document type not found: {}", e)))?;

    // Execute query
    let document_result: Option<Document> = Document::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch document: {}", e)))?;

    match document_result {
        Some(doc) => {
            let response = DocumentResponse::from_document(&doc, &data_contract, document_type)?;

            // Use json_compatible serializer to convert maps to objects
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response
                .serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        }
        None => Ok(JsValue::NULL),
    }
}

#[wasm_bindgen]
pub async fn get_document_with_proof_info(
    sdk: &WasmSdk,
    data_contract_id: &str,
    document_type: &str,
    document_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;

    // Parse IDs
    let contract_id = Identifier::from_string(
        data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let doc_id = Identifier::from_string(
        document_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create document query
    let query = DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, document_type)
        .await
        .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);

    // Fetch the data contract to get the document type
    let data_contract = dash_sdk::platform::DataContract::fetch(sdk.as_ref(), contract_id)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch data contract: {}", e)))?
        .ok_or_else(|| JsError::new("Data contract not found"))?;

    // Get the document type
    let document_type_ref = data_contract
        .document_type_for_name(document_type)
        .map_err(|e| JsError::new(&format!("Document type not found: {}", e)))?;

    // Execute query with proof
    let (document_result, metadata, proof) =
        Document::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await
            .map_err(|e| JsError::new(&format!("Failed to fetch document: {}", e)))?;

    match document_result {
        Some(doc) => {
            let doc_response =
                DocumentResponse::from_document(&doc, &data_contract, document_type_ref)?;

            let response = ProofMetadataResponse {
                data: doc_response,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response
                .serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        }
        None => {
            // Return null data with proof
            let response = ProofMetadataResponse {
                data: Option::<DocumentResponse>::None,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response
                .serialize(&serializer)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        }
    }
}

#[wasm_bindgen]
pub async fn get_dpns_usernames(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use dash_sdk::platform::FetchMany;
    use drive_proof_verifier::types::Documents;

    // DPNS contract ID on testnet
    const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    const DPNS_DOCUMENT_TYPE: &str = "domain";

    // Parse identity ID
    let identity_id_parsed = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Parse DPNS contract ID
    let contract_id = Identifier::from_string(
        DPNS_CONTRACT_ID,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create document query for DPNS domains owned by this identity
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, DPNS_DOCUMENT_TYPE)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Query by records.identity using the identityId index
    let where_clause = WhereClause {
        field: "records.identity".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Identifier(identity_id_parsed.to_buffer()),
    };

    query = query.with_where(where_clause);

    // Set limit from parameter or default to 10
    query.limit = limit.unwrap_or(10);

    // Execute query
    let documents_result: Documents = Document::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch DPNS documents: {}", e)))?;

    // Collect all usernames
    let mut usernames: Vec<String> = Vec::new();

    // Process all results
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            // Extract the username from the document
            let properties = doc.properties();

            if let (Some(Value::Text(label)), Some(Value::Text(parent_domain))) = (
                properties.get("label"),
                properties.get("normalizedParentDomainName"),
            ) {
                // Construct the full username
                let username = format!("{}.{}", label, parent_domain);
                usernames.push(username);
            }
        }
    }

    // Return usernames as a JSON array
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    usernames
        .serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize usernames: {}", e)))
}

// Keep the old function for backward compatibility but have it call the new one
#[wasm_bindgen]
pub async fn get_dpns_username(sdk: &WasmSdk, identity_id: &str) -> Result<JsValue, JsError> {
    // Call the new function with limit 1
    let result = get_dpns_usernames(sdk, identity_id, Some(1)).await?;

    // Extract the first username from the array
    if let Some(array) = result.dyn_ref::<js_sys::Array>() {
        if array.length() > 0 {
            return Ok(array.get(0));
        }
    }

    Ok(JsValue::NULL)
}

// Proof info versions for DPNS queries

#[wasm_bindgen]
pub async fn get_dpns_usernames_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
    limit: Option<u32>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use dash_sdk::platform::FetchMany;

    // DPNS contract ID on testnet
    const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    const DPNS_DOCUMENT_TYPE: &str = "domain";

    // Parse identity ID
    let identity_id_parsed = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Parse DPNS contract ID
    let contract_id = Identifier::from_string(
        DPNS_CONTRACT_ID,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create document query for DPNS domains owned by this identity
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, DPNS_DOCUMENT_TYPE)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Query by records.identity using the identityId index
    let where_clause = WhereClause {
        field: "records.identity".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Identifier(identity_id_parsed.to_buffer()),
    };

    query = query.with_where(where_clause);

    // Set limit from parameter or default to 10
    query.limit = limit.unwrap_or(10);

    // Execute query with proof
    let (documents_result, metadata, proof) =
        Document::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await
            .map_err(|e| {
                JsError::new(&format!("Failed to fetch DPNS documents with proof: {}", e))
            })?;

    // Collect all usernames
    let mut usernames: Vec<String> = Vec::new();

    // Process all results
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            // Extract the username from the document
            let properties = doc.properties();

            if let (Some(Value::Text(label)), Some(Value::Text(parent_domain))) = (
                properties.get("label"),
                properties.get("normalizedParentDomainName"),
            ) {
                // Construct the full username
                let username = format!("{}.{}", label, parent_domain);
                usernames.push(username);
            }
        }
    }

    let response = ProofMetadataResponse {
        data: usernames,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_dpns_username_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<JsValue, JsError> {
    // Call the new function with limit 1
    let result = get_dpns_usernames_with_proof_info(sdk, identity_id, Some(1)).await?;

    // The result already contains proof info, just modify the data field
    // Parse the result to extract first username
    let result_obj: serde_json::Value = serde_wasm_bindgen::from_value(result.clone())?;

    if let Some(data_array) = result_obj.get("data").and_then(|d| d.as_array()) {
        if let Some(first_username) = data_array.first() {
            // Create a new response with just the first username
            let mut modified_result = result_obj.clone();
            modified_result["data"] = first_username.clone();

            return serde_wasm_bindgen::to_value(&modified_result)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)));
        }
    }

    // If no username found, return null data with proof info
    let mut modified_result = result_obj.clone();
    modified_result["data"] = serde_json::Value::Null;

    serde_wasm_bindgen::to_value(&modified_result)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}
