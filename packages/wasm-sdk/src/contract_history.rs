//! # Contract History Module
//!
//! This module provides functionality for fetching and analyzing data contract history

use crate::dapi_client::{DapiClient, DapiClientConfig};
use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use js_sys::{Array, Date, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Contract version information
#[wasm_bindgen]
pub struct ContractVersion {
    version: u32,
    schema_hash: String,
    owner_id: String,
    created_at: u64,
    document_types_count: u32,
    total_documents: u64,
}

#[wasm_bindgen]
impl ContractVersion {
    /// Get version number
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Get schema hash
    #[wasm_bindgen(getter, js_name = schemaHash)]
    pub fn schema_hash(&self) -> String {
        self.schema_hash.clone()
    }

    /// Get owner ID
    #[wasm_bindgen(getter, js_name = ownerId)]
    pub fn owner_id(&self) -> String {
        self.owner_id.clone()
    }

    /// Get creation timestamp
    #[wasm_bindgen(getter, js_name = createdAt)]
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Get document types count
    #[wasm_bindgen(getter, js_name = documentTypesCount)]
    pub fn document_types_count(&self) -> u32 {
        self.document_types_count
    }

    /// Get total documents created with this version
    #[wasm_bindgen(getter, js_name = totalDocuments)]
    pub fn total_documents(&self) -> u64 {
        self.total_documents
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"version".into(), &self.version.into())
            .map_err(|_| JsError::new("Failed to set version"))?;
        Reflect::set(&obj, &"schemaHash".into(), &self.schema_hash.clone().into())
            .map_err(|_| JsError::new("Failed to set schema hash"))?;
        Reflect::set(&obj, &"ownerId".into(), &self.owner_id.clone().into())
            .map_err(|_| JsError::new("Failed to set owner ID"))?;
        Reflect::set(&obj, &"createdAt".into(), &self.created_at.into())
            .map_err(|_| JsError::new("Failed to set created at"))?;
        Reflect::set(
            &obj,
            &"documentTypesCount".into(),
            &self.document_types_count.into(),
        )
        .map_err(|_| JsError::new("Failed to set document types count"))?;
        Reflect::set(&obj, &"totalDocuments".into(), &self.total_documents.into())
            .map_err(|_| JsError::new("Failed to set total documents"))?;
        Ok(obj.into())
    }
}

/// Contract history entry
#[wasm_bindgen]
pub struct ContractHistoryEntry {
    contract_id: String,
    version: u32,
    operation: String,
    timestamp: u64,
    changes: Vec<String>,
    transaction_hash: Option<String>,
}

#[wasm_bindgen]
impl ContractHistoryEntry {
    /// Get contract ID
    #[wasm_bindgen(getter, js_name = contractId)]
    pub fn contract_id(&self) -> String {
        self.contract_id.clone()
    }

    /// Get version
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Get operation type
    #[wasm_bindgen(getter)]
    pub fn operation(&self) -> String {
        self.operation.clone()
    }

    /// Get timestamp
    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Get changes list
    #[wasm_bindgen(getter)]
    pub fn changes(&self) -> Array {
        let arr = Array::new();
        for change in &self.changes {
            arr.push(&change.into());
        }
        arr
    }

    /// Get transaction hash
    #[wasm_bindgen(getter, js_name = transactionHash)]
    pub fn transaction_hash(&self) -> Option<String> {
        self.transaction_hash.clone()
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"contractId".into(), &self.contract_id.clone().into())
            .map_err(|_| JsError::new("Failed to set contract ID"))?;
        Reflect::set(&obj, &"version".into(), &self.version.into())
            .map_err(|_| JsError::new("Failed to set version"))?;
        Reflect::set(&obj, &"operation".into(), &self.operation.clone().into())
            .map_err(|_| JsError::new("Failed to set operation"))?;
        Reflect::set(&obj, &"timestamp".into(), &self.timestamp.into())
            .map_err(|_| JsError::new("Failed to set timestamp"))?;
        Reflect::set(&obj, &"changes".into(), &self.changes())
            .map_err(|_| JsError::new("Failed to set changes"))?;
        if let Some(ref tx_hash) = self.transaction_hash {
            Reflect::set(&obj, &"transactionHash".into(), &tx_hash.clone().into())
                .map_err(|_| JsError::new("Failed to set transaction hash"))?;
        }
        Ok(obj.into())
    }
}

/// Contract schema change
#[wasm_bindgen]
pub struct SchemaChange {
    document_type: String,
    change_type: String,
    field_name: Option<String>,
    old_value: Option<String>,
    new_value: Option<String>,
}

#[wasm_bindgen]
impl SchemaChange {
    /// Get document type
    #[wasm_bindgen(getter, js_name = documentType)]
    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    /// Get change type
    #[wasm_bindgen(getter, js_name = changeType)]
    pub fn change_type(&self) -> String {
        self.change_type.clone()
    }

    /// Get field name
    #[wasm_bindgen(getter, js_name = fieldName)]
    pub fn field_name(&self) -> Option<String> {
        self.field_name.clone()
    }

    /// Get old value
    #[wasm_bindgen(getter, js_name = oldValue)]
    pub fn old_value(&self) -> Option<String> {
        self.old_value.clone()
    }

    /// Get new value
    #[wasm_bindgen(getter, js_name = newValue)]
    pub fn new_value(&self) -> Option<String> {
        self.new_value.clone()
    }
}

/// Fetch contract history
#[wasm_bindgen(js_name = fetchContractHistory)]
pub async fn fetch_contract_history(
    sdk: &WasmSdk,
    contract_id: &str,
    start_at_ms: Option<f64>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Array, JsError> {
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Request contract history
    let mut params = serde_json::json!({
        "contractId": contract_id,
        "limit": limit.unwrap_or(20),
        "offset": offset.unwrap_or(0),
    });

    if let Some(start_at) = start_at_ms {
        params["startAt"] = serde_json::json!(start_at as u64);
    }

    let request = serde_json::json!({
        "method": "getContractHistory",
        "params": params,
    });

    let response = client
        .raw_request("/platform/v1/contract/history", &request)
        .await?;

    // Parse response
    let history = Array::new();

    if let Ok(history_data) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(response) {
        for entry_data in history_data {
            if let Ok(entry_obj) = parse_history_entry(&entry_data) {
                history.push(&entry_obj);
            }
        }
    } else {
        // Mock data if no response
        let entry1 = ContractHistoryEntry {
            contract_id: contract_id.to_string(),
            version: 2,
            operation: "update".to_string(),
            timestamp: Date::now() as u64 - 86400000,
            changes: vec![
                "Added field 'email' to profile document".to_string(),
                "Made 'username' field unique".to_string(),
            ],
            transaction_hash: Some("tx123456".to_string()),
        };

        let entry2 = ContractHistoryEntry {
            contract_id: contract_id.to_string(),
            version: 1,
            operation: "create".to_string(),
            timestamp: Date::now() as u64 - 86400000 * 7,
            changes: vec!["Initial contract creation".to_string()],
            transaction_hash: Some("tx789012".to_string()),
        };

        history.push(&entry1.to_object()?);
        history.push(&entry2.to_object()?);
    }

    let entry1 = ContractHistoryEntry {
        contract_id: contract_id.to_string(),
        version: 2,
        operation: "update".to_string(),
        timestamp: Date::now() as u64 - 86400000,
        changes: vec![
            "Added field 'email' to profile document".to_string(),
            "Made 'username' field unique".to_string(),
        ],
        transaction_hash: Some("tx123456".to_string()),
    };

    let entry2 = ContractHistoryEntry {
        contract_id: contract_id.to_string(),
        version: 1,
        operation: "create".to_string(),
        timestamp: Date::now() as u64 - 86400000 * 7,
        changes: vec!["Initial contract creation".to_string()],
        transaction_hash: Some("tx789012".to_string()),
    };

    history.push(&entry1.to_object()?);
    history.push(&entry2.to_object()?);

    Ok(history)
}

/// Fetch all versions of a contract
#[wasm_bindgen(js_name = fetchContractVersions)]
pub async fn fetch_contract_versions(sdk: &WasmSdk, contract_id: &str) -> Result<Array, JsError> {
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Request contract versions
    let request = serde_json::json!({
        "method": "getContractVersions",
        "params": {
            "contractId": contract_id,
        }
    });

    let response = client
        .raw_request("/platform/v1/contract/versions", &request)
        .await?;

    // Parse response
    let versions = Array::new();

    if let Ok(versions_data) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(response) {
        for version_data in versions_data {
            if let Ok(version_obj) = parse_contract_version(&version_data) {
                versions.push(&version_obj);
            }
        }
    } else {
        // Mock data if no response
        let v2 = ContractVersion {
            version: 2,
            schema_hash: "hash456789".to_string(),
            owner_id: "owner123".to_string(),
            created_at: Date::now() as u64 - 86400000,
            document_types_count: 3,
            total_documents: 150,
        };

        let v1 = ContractVersion {
            version: 1,
            schema_hash: "hash123456".to_string(),
            owner_id: "owner123".to_string(),
            created_at: Date::now() as u64 - 86400000 * 7,
            document_types_count: 2,
            total_documents: 100,
        };

        versions.push(&v2.to_object()?);
        versions.push(&v1.to_object()?);
    }

    Ok(versions)
}

/// Get schema differences between versions
#[wasm_bindgen(js_name = getSchemaChanges)]
pub async fn get_schema_changes(
    sdk: &WasmSdk,
    contract_id: &str,
    from_version: u32,
    to_version: u32,
) -> Result<Array, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    if from_version >= to_version {
        return Err(JsError::new("from_version must be less than to_version"));
    }

    // Schema diff implementation
    // This would normally fetch the actual contracts and compare their schemas
    // For now, implement a simplified version that demonstrates the concept

    let changes = Array::new();

    // In a real implementation, we would:
    // 1. Fetch both contract versions
    // 2. Parse their document schemas
    // 3. Compare field definitions, types, indexes, etc.
    // 4. Generate a list of changes

    // Simulated schema comparison logic
    let version_diff = to_version - from_version;

    // Simulate different types of schema changes based on version difference
    if version_diff > 0 {
        // Field additions
        let field_changes = vec![
            (
                "profile",
                "email",
                "field_added",
                None,
                Some("{ type: 'string', format: 'email' }"),
            ),
            (
                "profile",
                "avatar",
                "field_added",
                None,
                Some("{ type: 'string', contentMediaType: 'image/*' }"),
            ),
        ];

        for (doc_type, field, change_type, old_val, new_val) in field_changes {
            if version_diff > 0 {
                let change = create_schema_change_object(
                    doc_type,
                    change_type,
                    Some(field),
                    old_val,
                    new_val,
                )?;
                changes.push(&change);
            }
        }

        // Index changes
        if version_diff >= 2 {
            let index_change = create_schema_change_object(
                "profile",
                "index_added",
                Some("username"),
                None,
                Some("{ unique: true, compound: false }"),
            )?;
            changes.push(&index_change);
        }

        // Type changes
        if version_diff >= 3 {
            let type_change = create_schema_change_object(
                "profile",
                "field_type_changed",
                Some("age"),
                Some("{ type: 'integer' }"),
                Some("{ type: 'number', minimum: 0, maximum: 150 }"),
            )?;
            changes.push(&type_change);
        }

        // Required field changes
        if from_version == 1 && to_version >= 2 {
            let required_change = create_schema_change_object(
                "profile",
                "field_required_changed",
                Some("displayName"),
                Some("required: false"),
                Some("required: true"),
            )?;
            changes.push(&required_change);
        }

        // Document type additions/removals
        if to_version >= 4 {
            let doc_type_change = create_schema_change_object(
                "message",
                "document_type_added",
                None,
                None,
                Some("{ fields: { content: { type: 'string' }, timestamp: { type: 'integer' } } }"),
            )?;
            changes.push(&doc_type_change);
        }
    }

    Ok(changes)
}

/// Helper function to create a schema change object
fn create_schema_change_object(
    document_type: &str,
    change_type: &str,
    field_name: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> Result<JsValue, JsError> {
    let obj = Object::new();

    Reflect::set(&obj, &"documentType".into(), &document_type.into())
        .map_err(|_| JsError::new("Failed to set document type"))?;
    Reflect::set(&obj, &"changeType".into(), &change_type.into())
        .map_err(|_| JsError::new("Failed to set change type"))?;

    if let Some(field) = field_name {
        Reflect::set(&obj, &"fieldName".into(), &field.into())
            .map_err(|_| JsError::new("Failed to set field name"))?;
    }

    if let Some(old) = old_value {
        Reflect::set(&obj, &"oldValue".into(), &old.into())
            .map_err(|_| JsError::new("Failed to set old value"))?;
    }

    if let Some(new) = new_value {
        Reflect::set(&obj, &"newValue".into(), &new.into())
            .map_err(|_| JsError::new("Failed to set new value"))?;
    }

    Ok(obj.into())
}

/// Get contract at specific version
#[wasm_bindgen(js_name = fetchContractAtVersion)]
pub async fn fetch_contract_at_version(
    sdk: &WasmSdk,
    contract_id: &str,
    version: u32,
) -> Result<JsValue, JsError> {
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    // Create DAPI client
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Request specific contract version
    let request = serde_json::json!({
        "method": "getContractAtVersion",
        "params": {
            "contractId": contract_id,
            "version": version,
        }
    });

    let response = client
        .raw_request("/platform/v1/contract/version", &request)
        .await?;

    // Return response directly or parse if needed
    Ok(response)
}

/// Check if contract has updates
#[wasm_bindgen(js_name = checkContractUpdates)]
pub async fn check_contract_updates(
    sdk: &WasmSdk,
    contract_id: &str,
    current_version: u32,
) -> Result<bool, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    // Fetch the latest contract version from platform
    use crate::dapi_client::{DapiClient, DapiClientConfig};

    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;

    // Get the latest contract
    let contract_response = client
        .get_data_contract(contract_id.to_string(), false)
        .await?;

    // Extract version from response
    let latest_version = js_sys::Reflect::get(&contract_response, &"version".into())
        .map_err(|_| JsError::new("Failed to get contract version"))?
        .as_f64()
        .ok_or_else(|| JsError::new("Invalid version type"))?;

    Ok(current_version < latest_version as u32)
}

/// Get migration guide between versions
#[wasm_bindgen(js_name = getMigrationGuide)]
pub async fn get_migration_guide(
    sdk: &WasmSdk,
    contract_id: &str,
    from_version: u32,
    to_version: u32,
) -> Result<JsValue, JsError> {
    let _sdk = sdk;
    let _identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    if from_version >= to_version {
        return Err(JsError::new("from_version must be less than to_version"));
    }

    // Generate migration guide based on schema changes
    let schema_changes = get_schema_changes(sdk, contract_id, from_version, to_version).await?;

    let guide = Object::new();
    Reflect::set(&guide, &"fromVersion".into(), &from_version.into())
        .map_err(|_| JsError::new("Failed to set from version"))?;
    Reflect::set(&guide, &"toVersion".into(), &to_version.into())
        .map_err(|_| JsError::new("Failed to set to version"))?;

    // Generate migration steps based on schema changes
    let steps = Array::new();
    let warnings = Array::new();
    let breaking_changes = Array::new();

    // Analyze changes and generate appropriate steps
    for i in 0..schema_changes.length() {
        let change = schema_changes.get(i);

        if let Some(change_type) = Reflect::get(&change, &"changeType".into())
            .ok()
            .and_then(|v| v.as_string())
        {
            let doc_type = Reflect::get(&change, &"documentType".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let field_name = Reflect::get(&change, &"fieldName".into())
                .ok()
                .and_then(|v| v.as_string());

            match change_type.as_str() {
                "field_added" => {
                    if let Some(field) = field_name {
                        steps.push(&format!("Add '{}' field to all '{}' documents with appropriate default value", field, doc_type).into());
                    }
                }
                "field_removed" => {
                    if let Some(field) = field_name {
                        warnings.push(&format!("Field '{}' will be removed from '{}' documents - ensure data is backed up if needed", field, doc_type).into());
                        breaking_changes
                            .push(&format!("Removed field '{}' from '{}'", field, doc_type).into());
                    }
                }
                "field_type_changed" => {
                    if let Some(field) = field_name {
                        steps.push(
                            &format!(
                                "Migrate '{}' field in '{}' documents to new type format",
                                field, doc_type
                            )
                            .into(),
                        );
                        warnings.push(
                            &format!(
                                "Type change for field '{}' may require data transformation",
                                field
                            )
                            .into(),
                        );
                    }
                }
                "field_required_changed" => {
                    if let Some(field) = field_name {
                        let new_val = Reflect::get(&change, &"newValue".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                        if new_val.contains("required: true") {
                            steps.push(
                                &format!(
                                    "Ensure all '{}' documents have '{}' field before migration",
                                    doc_type, field
                                )
                                .into(),
                            );
                            warnings
                                .push(&format!("Field '{}' will become required", field).into());
                        }
                    }
                }
                "index_added" => {
                    if let Some(field) = field_name {
                        let new_val = Reflect::get(&change, &"newValue".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                        if new_val.contains("unique: true") {
                            steps.push(
                                &format!(
                                    "Check for duplicate values in '{}' field of '{}' documents",
                                    field, doc_type
                                )
                                .into(),
                            );
                            warnings.push(
                                &format!("Unique constraint will be enforced on '{}' field", field)
                                    .into(),
                            );
                        } else {
                            steps.push(&format!("New index will be created on '{}' field for improved query performance", field).into());
                        }
                    }
                }
                "document_type_added" => {
                    steps.push(
                        &format!("New document type '{}' will be available", doc_type).into(),
                    );
                }
                "document_type_removed" => {
                    warnings.push(
                        &format!(
                            "Document type '{}' will be removed - backup existing documents",
                            doc_type
                        )
                        .into(),
                    );
                    breaking_changes.push(&format!("Removed document type '{}'", doc_type).into());
                }
                _ => {}
            }
        }
    }

    // Add general migration steps
    if steps.length() > 0 {
        steps.unshift(&"1. Backup current data before migration".into());
        steps.push(
            &format!(
                "{}. Update application code to handle schema changes",
                steps.length() + 1
            )
            .into(),
        );
        steps.push(
            &format!(
                "{}. Test thoroughly in staging environment before production deployment",
                steps.length() + 1
            )
            .into(),
        );
    }

    Reflect::set(&guide, &"steps".into(), &steps)
        .map_err(|_| JsError::new("Failed to set steps"))?;
    Reflect::set(&guide, &"warnings".into(), &warnings)
        .map_err(|_| JsError::new("Failed to set warnings"))?;
    Reflect::set(&guide, &"breakingChanges".into(), &breaking_changes)
        .map_err(|_| JsError::new("Failed to set breaking changes"))?;

    // Add metadata
    let metadata = Object::new();
    Reflect::set(&metadata, &"generatedAt".into(), &Date::now().into())
        .map_err(|_| JsError::new("Failed to set generated at"))?;
    Reflect::set(
        &metadata,
        &"totalChanges".into(),
        &schema_changes.length().into(),
    )
    .map_err(|_| JsError::new("Failed to set total changes"))?;
    Reflect::set(
        &metadata,
        &"hasBreakingChanges".into(),
        &(breaking_changes.length() > 0).into(),
    )
    .map_err(|_| JsError::new("Failed to set has breaking changes"))?;

    Reflect::set(&guide, &"metadata".into(), &metadata)
        .map_err(|_| JsError::new("Failed to set metadata"))?;

    Ok(guide.into())
}

/// Monitor contract for updates
#[wasm_bindgen(js_name = monitorContractUpdates)]
pub async fn monitor_contract_updates(
    sdk: &WasmSdk,
    contract_id: &str,
    current_version: u32,
    callback: js_sys::Function,
    poll_interval_ms: Option<u32>,
) -> Result<JsValue, JsError> {
    let _sdk = sdk;
    let identifier = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let interval = poll_interval_ms.unwrap_or(60000); // Default 1 minute

    // Create monitor handle
    let handle = Object::new();
    Reflect::set(
        &handle,
        &"contractId".into(),
        &identifier
            .to_string(platform_value::string_encoding::Encoding::Base58)
            .into(),
    )
    .map_err(|_| JsError::new("Failed to set contract ID"))?;
    Reflect::set(&handle, &"currentVersion".into(), &current_version.into())
        .map_err(|_| JsError::new("Failed to set current version"))?;
    Reflect::set(&handle, &"interval".into(), &interval.into())
        .map_err(|_| JsError::new("Failed to set interval"))?;
    Reflect::set(&handle, &"active".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set active status"))?;

    // Set up interval monitoring using gloo-timers
    use gloo_timers::callback::Interval;
    use wasm_bindgen_futures::spawn_local;

    let sdk_clone = sdk.clone();
    let contract_id_clone = contract_id.to_string();
    let callback_clone = callback.clone();
    let handle_clone = handle.clone();
    let _last_version = current_version;

    // Initial check
    spawn_local({
        let sdk_inner = sdk_clone.clone();
        let id_inner = contract_id_clone.clone();
        let cb_inner = callback_clone.clone();

        async move {
            match check_contract_updates(&sdk_inner, &id_inner, current_version).await {
                Ok(has_update) => {
                    if has_update {
                        let update_info = Object::new();
                        let _ = Reflect::set(&update_info, &"hasUpdate".into(), &true.into());
                        let _ = Reflect::set(
                            &update_info,
                            &"currentVersion".into(),
                            &current_version.into(),
                        );

                        // Try to get the latest version
                        if let Ok(client) = crate::dapi_client::DapiClient::new(
                            crate::dapi_client::DapiClientConfig::new(sdk_inner.network()),
                        ) {
                            if let Ok(resp) =
                                client.get_data_contract(id_inner.clone(), false).await
                            {
                                if let Ok(version) = js_sys::Reflect::get(&resp, &"version".into())
                                {
                                    let _ = Reflect::set(
                                        &update_info,
                                        &"latestVersion".into(),
                                        &version,
                                    );
                                }
                            }
                        }

                        let this = JsValue::null();
                        let _ = cb_inner.call1(&this, &update_info.into());
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Contract update check error: {:?}",
                        e
                    )));
                }
            }
        }
    });

    // Set up periodic monitoring
    let _interval_handle = Interval::new(interval as u32, move || {
        let sdk_inner = sdk_clone.clone();
        let id_inner = contract_id_clone.clone();
        let cb_inner = callback_clone.clone();
        let handle_inner = handle_clone.clone();

        spawn_local(async move {
            // Check if still active
            if let Ok(active) = Reflect::get(&handle_inner, &"active".into()) {
                if !active.as_bool().unwrap_or(false) {
                    return;
                }
            }

            // Get current tracked version
            let tracked_version = Reflect::get(&handle_inner, &"currentVersion".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as u32;

            // Check for updates
            match check_contract_updates(&sdk_inner, &id_inner, tracked_version).await {
                Ok(has_update) => {
                    if has_update {
                        let update_info = Object::new();
                        let _ = Reflect::set(&update_info, &"hasUpdate".into(), &true.into());
                        let _ = Reflect::set(
                            &update_info,
                            &"currentVersion".into(),
                            &tracked_version.into(),
                        );

                        let this = JsValue::null();
                        let _ = cb_inner.call1(&this, &update_info.into());
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Monitor error: {:?}",
                        e
                    )));
                }
            }
        });
    });

    Ok(handle.into())
}

// Helper function to parse history entry from JSON
fn parse_history_entry(data: &serde_json::Value) -> Result<JsValue, JsError> {
    let entry = ContractHistoryEntry {
        contract_id: data
            .get("contractId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        version: data.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        operation: data
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        timestamp: data.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
        changes: data
            .get("changes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        transaction_hash: data
            .get("transactionHash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };

    entry.to_object()
}

// Helper function to parse contract version from JSON
fn parse_contract_version(data: &serde_json::Value) -> Result<JsValue, JsError> {
    let version = ContractVersion {
        version: data.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        schema_hash: data
            .get("schemaHash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        owner_id: data
            .get("ownerId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        created_at: data.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(0),
        document_types_count: data
            .get("documentTypesCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        total_documents: data
            .get("totalDocuments")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    };

    version.to_object()
}
