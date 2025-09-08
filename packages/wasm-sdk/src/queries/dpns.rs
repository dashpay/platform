use crate::sdk::WasmSdk;
use crate::queries::ProofMetadataResponse;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{FetchMany, Document};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::document::DocumentV0Getters;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DpnsUsernameInfo {
    username: String,
    identity_id: String,
    document_id: String,
}

#[wasm_bindgen]
pub async fn get_dpns_username_by_name(sdk: &WasmSdk, username: &str) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use drive::query::{WhereClause, WhereOperator};
    use dash_sdk::dpp::platform_value::Value;

    // DPNS contract ID on testnet
    const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    const DPNS_DOCUMENT_TYPE: &str = "domain";

    // Parse username into label and domain
    let parts: Vec<&str> = username.split('.').collect();
    if parts.len() != 2 {
        return Err(JsError::new(
            "Invalid username format. Expected format: label.dash",
        ));
    }
    let label = parts[0];
    let domain = parts[1];

    // Parse DPNS contract ID
    let contract_id =
        dash_sdk::dpp::prelude::Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58)?;

    // Create document query
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, DPNS_DOCUMENT_TYPE)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Query by label and normalizedParentDomainName
    query = query.with_where(WhereClause {
        field: "normalizedLabel".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(label.to_lowercase()),
    });

    query = query.with_where(WhereClause {
        field: "normalizedParentDomainName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(domain.to_lowercase()),
    });

    let documents = Document::fetch_many(sdk.as_ref(), query).await?;

    if let Some((_, Some(document))) = documents.into_iter().next() {
        let result = DpnsUsernameInfo {
            username: username.to_string(),
            identity_id: document.owner_id().to_string(Encoding::Base58),
            document_id: document.id().to_string(Encoding::Base58),
        };

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Err(JsError::new(&format!("Username '{}' not found", username)))
    }
}

#[wasm_bindgen]
pub async fn get_dpns_username_by_name_with_proof_info(
    sdk: &WasmSdk,
    username: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use drive::query::{WhereClause, WhereOperator};
    use dash_sdk::dpp::platform_value::Value;

    // DPNS contract ID on testnet
    const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    const DPNS_DOCUMENT_TYPE: &str = "domain";

    // Parse username into label and domain
    let parts: Vec<&str> = username.split('.').collect();
    if parts.len() != 2 {
        return Err(JsError::new(
            "Invalid username format. Expected format: label.dash",
        ));
    }
    let label = parts[0];
    let domain = parts[1];

    // Parse DPNS contract ID
    let contract_id =
        dash_sdk::dpp::prelude::Identifier::from_string(DPNS_CONTRACT_ID, Encoding::Base58)?;

    // Create document query
    let mut query =
        DocumentQuery::new_with_data_contract_id(sdk.as_ref(), contract_id, DPNS_DOCUMENT_TYPE)
            .await
            .map_err(|e| JsError::new(&format!("Failed to create document query: {}", e)))?;

    // Query by label and normalizedParentDomainName
    query = query.with_where(WhereClause {
        field: "normalizedLabel".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(label.to_lowercase()),
    });

    query = query.with_where(WhereClause {
        field: "normalizedParentDomainName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text(domain.to_lowercase()),
    });

    let (documents, metadata, proof) =
        Document::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None).await?;

    if let Some((_, Some(document))) = documents.into_iter().next() {
        let result = DpnsUsernameInfo {
            username: username.to_string(),
            identity_id: document.owner_id().to_string(Encoding::Base58),
            document_id: document.id().to_string(Encoding::Base58),
        };

        let response = ProofMetadataResponse {
            data: result,
            metadata: metadata.into(),
            proof: proof.into(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response
            .serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        Err(JsError::new(&format!("Username '{}' not found", username)))
    }
}
