use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::Fetch;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::document::Document;
use serde_json::Value as JsonValue;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DocumentResponse {
    id: String,
    owner_id: String,
    revision: u64,
    created_at: Option<u64>,
    updated_at: Option<u64>,
    transferred_at: Option<u64>,
    created_at_block_height: Option<u64>,
    updated_at_block_height: Option<u64>,
    transferred_at_block_height: Option<u64>,
    created_at_core_block_height: Option<u32>,
    updated_at_core_block_height: Option<u32>,
    transferred_at_core_block_height: Option<u32>,
    data: serde_json::Map<String, JsonValue>,
}

impl DocumentResponse {
    fn from_document(doc: &Document) -> Result<Self, JsError> {
        use dash_sdk::dpp::document::DocumentV0Getters;
        
        // Convert the document properties to JSON
        let mut data = serde_json::Map::new();
        
        // Get document properties directly
        // Use the built-in try_into() method from platform_value
        for (key, value) in doc.properties() {
            let json_value: JsonValue = value.clone().try_into()
                .map_err(|e| JsError::new(&format!("Failed to convert value to JSON: {:?}", e)))?;
            data.insert(key.clone(), json_value);
        }
        
        Ok(Self {
            id: doc.id().to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
            owner_id: doc.owner_id().to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58),
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
        })
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
    let mut query = DocumentQuery::new_with_data_contract_id(
        sdk.as_ref(),
        contract_id,
        document_type,
    )
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
    
    // Note: where_clause and order_by parsing would require WhereClause and OrderClause
    // which are not fully exposed in the SDK. For now, we ignore these parameters.
    if where_clause.is_some() || order_by.is_some() {
        // Log warning about unsupported features
        web_sys::console::warn_1(&JsValue::from_str(
            "Warning: where and orderBy clauses are not yet fully supported in the WASM SDK"
        ));
    }
    
    // Execute query
    let documents_result: Documents = Document::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch documents: {}", e)))?;
    
    // Convert documents to response format
    let mut responses: Vec<DocumentResponse> = Vec::new();
    for (_, doc_opt) in documents_result {
        if let Some(doc) = doc_opt {
            responses.push(DocumentResponse::from_document(&doc)?);
        }
    }
    
    serde_wasm_bindgen::to_value(&responses)
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
    let query = DocumentQuery::new_with_data_contract_id(
        sdk.as_ref(),
        contract_id,
        document_type,
    )
    .await
    .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?
    .with_document_id(&doc_id);
    
    // Execute query
    let document_result: Option<Document> = Document::fetch(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch document: {}", e)))?;
    
    match document_result {
        Some(doc) => {
            let response = DocumentResponse::from_document(&doc)?;
            serde_wasm_bindgen::to_value(&response)
                .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
        },
        None => Ok(JsValue::NULL),
    }
}