//! # Query Module
//!
//! This module provides WASM-compatible query types for fetching data from Platform.
//! Queries are used to specify search criteria when fetching objects.
//!
//! ## Example
//!
//! ```javascript
//! const query = new IdentifierQuery("base58_encoded_id");
//! const identity = await fetchIdentity(sdk, query);
//! ```

use js_sys::{Array, Object, Reflect};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Query by identifier
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct IdentifierQuery {
    identifier: Identifier,
}

#[wasm_bindgen]
impl IdentifierQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> Result<IdentifierQuery, JsError> {
        let identifier =
            Identifier::from_string(id, platform_value::string_encoding::Encoding::Base58)
                .map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;

        Ok(IdentifierQuery { identifier })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.identifier
            .to_string(platform_value::string_encoding::Encoding::Base58)
    }
}

impl IdentifierQuery {
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }
}

/// Query for multiple identifiers
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct IdentifiersQuery {
    identifiers: Vec<Identifier>,
}

#[wasm_bindgen]
impl IdentifiersQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(ids: Vec<String>) -> Result<IdentifiersQuery, JsError> {
        let identifiers: Result<Vec<Identifier>, _> = ids
            .iter()
            .map(|id| {
                Identifier::from_string(id, platform_value::string_encoding::Encoding::Base58)
            })
            .collect();

        let identifiers =
            identifiers.map_err(|e| JsError::new(&format!("Invalid identifier: {}", e)))?;

        Ok(IdentifiersQuery { identifiers })
    }

    #[wasm_bindgen(getter)]
    pub fn ids(&self) -> Vec<String> {
        self.identifiers
            .iter()
            .map(|id| id.to_string(platform_value::string_encoding::Encoding::Base58))
            .collect()
    }

    #[wasm_bindgen(getter)]
    pub fn count(&self) -> usize {
        self.identifiers.len()
    }
}

impl IdentifiersQuery {
    pub fn identifiers(&self) -> &[Identifier] {
        &self.identifiers
    }
}

/// Query with limit and pagination support
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct LimitQuery {
    /// Maximum number of results to return
    limit: Option<u32>,
    /// Starting offset for pagination
    offset: Option<u32>,
    /// Starting key for cursor-based pagination
    start_key: Option<Vec<u8>>,
    /// Whether to include the start key in results
    start_included: bool,
}

#[wasm_bindgen]
impl LimitQuery {
    #[wasm_bindgen(constructor)]
    pub fn new() -> LimitQuery {
        LimitQuery {
            limit: None,
            offset: None,
            start_key: None,
            start_included: false,
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_limit(&mut self, limit: u32) {
        self.limit = Some(limit);
    }

    #[wasm_bindgen(setter)]
    pub fn set_offset(&mut self, offset: u32) {
        self.offset = Some(offset);
    }

    #[wasm_bindgen(setter, js_name = setStartKey)]
    pub fn set_start_key(&mut self, key: Vec<u8>) {
        self.start_key = Some(key);
    }

    #[wasm_bindgen(setter, js_name = setStartIncluded)]
    pub fn set_start_included(&mut self, included: bool) {
        self.start_included = included;
    }

    #[wasm_bindgen(getter)]
    pub fn limit(&self) -> Option<u32> {
        self.limit
    }

    #[wasm_bindgen(getter)]
    pub fn offset(&self) -> Option<u32> {
        self.offset
    }
}

/// Document query for searching documents
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct DocumentQuery {
    contract_id: Identifier,
    document_type: String,
    where_clauses: Vec<WhereClause>,
    order_by: Vec<OrderByClause>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhereClause {
    pub field: String,
    pub operator: WhereOperator,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WhereOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    In,
    NotIn,
    StartsWith,
    Contains,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderByClause {
    pub field: String,
    pub ascending: bool,
}

#[wasm_bindgen]
impl DocumentQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(contract_id: &str, document_type: &str) -> Result<DocumentQuery, JsError> {
        let contract_id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract identifier: {}", e)))?;

        Ok(DocumentQuery {
            contract_id,
            document_type: document_type.to_string(),
            where_clauses: vec![],
            order_by: vec![],
            limit: None,
            offset: None,
        })
    }

    #[wasm_bindgen(js_name = addWhereClause)]
    pub fn add_where_clause(
        &mut self,
        field: &str,
        operator: &str,
        value: JsValue,
    ) -> Result<(), JsError> {
        let operator = match operator {
            "==" | "equal" => WhereOperator::Equal,
            "!=" | "notEqual" => WhereOperator::NotEqual,
            ">" | "greaterThan" => WhereOperator::GreaterThan,
            ">=" | "greaterThanOrEqual" => WhereOperator::GreaterThanOrEqual,
            "<" | "lessThan" => WhereOperator::LessThan,
            "<=" | "lessThanOrEqual" => WhereOperator::LessThanOrEqual,
            "in" => WhereOperator::In,
            "notIn" => WhereOperator::NotIn,
            "startsWith" => WhereOperator::StartsWith,
            "contains" => WhereOperator::Contains,
            _ => return Err(JsError::new(&format!("Unknown operator: {}", operator))),
        };

        // Convert JsValue to serde_json::Value
        let value: serde_json::Value = serde_wasm_bindgen::from_value(value)
            .map_err(|e| JsError::new(&format!("Invalid value: {}", e)))?;

        self.where_clauses.push(WhereClause {
            field: field.to_string(),
            operator,
            value,
        });

        Ok(())
    }

    #[wasm_bindgen(js_name = addOrderBy)]
    pub fn add_order_by(&mut self, field: &str, ascending: bool) {
        self.order_by.push(OrderByClause {
            field: field.to_string(),
            ascending,
        });
    }

    #[wasm_bindgen(setter)]
    pub fn set_limit(&mut self, limit: u32) {
        self.limit = Some(limit);
    }

    #[wasm_bindgen(setter)]
    pub fn set_offset(&mut self, offset: u32) {
        self.offset = Some(offset);
    }

    #[wasm_bindgen(getter, js_name = contractId)]
    pub fn contract_id(&self) -> String {
        self.contract_id
            .to_string(platform_value::string_encoding::Encoding::Base58)
    }

    #[wasm_bindgen(getter, js_name = documentType)]
    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn limit(&self) -> Option<u32> {
        self.limit
    }

    #[wasm_bindgen(getter)]
    pub fn offset(&self) -> Option<u32> {
        self.offset
    }

    /// Get where clauses as JavaScript array
    #[wasm_bindgen(js_name = getWhereClauses)]
    pub fn get_where_clauses(&self) -> Result<Array, JsError> {
        let arr = Array::new();

        for clause in &self.where_clauses {
            let obj = Object::new();
            Reflect::set(&obj, &"field".into(), &clause.field.clone().into())
                .map_err(|_| JsError::new("Failed to set field"))?;
            Reflect::set(
                &obj,
                &"operator".into(),
                &format!("{:?}", clause.operator).into(),
            )
            .map_err(|_| JsError::new("Failed to set operator"))?;

            let value = serde_wasm_bindgen::to_value(&clause.value)
                .map_err(|e| JsError::new(&format!("Failed to convert value: {}", e)))?;
            Reflect::set(&obj, &"value".into(), &value)
                .map_err(|_| JsError::new("Failed to set value"))?;

            arr.push(&obj.into());
        }

        Ok(arr)
    }

    /// Get order by clauses as JavaScript array
    #[wasm_bindgen(js_name = getOrderByClauses)]
    pub fn get_order_by_clauses(&self) -> Result<Array, JsError> {
        let arr = Array::new();

        for clause in &self.order_by {
            let obj = Object::new();
            Reflect::set(&obj, &"field".into(), &clause.field.clone().into())
                .map_err(|_| JsError::new("Failed to set field"))?;
            Reflect::set(&obj, &"ascending".into(), &clause.ascending.into())
                .map_err(|_| JsError::new("Failed to set ascending"))?;
            arr.push(&obj.into());
        }

        Ok(arr)
    }
}

impl DocumentQuery {
    pub fn contract_identifier(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn where_clauses(&self) -> &[WhereClause] {
        &self.where_clauses
    }

    pub fn order_by(&self) -> &[OrderByClause] {
        &self.order_by
    }
}

/// Query for epochs
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct EpochQuery {
    start_epoch: Option<u32>,
    count: Option<u32>,
    ascending: bool,
}

#[wasm_bindgen]
impl EpochQuery {
    #[wasm_bindgen(constructor)]
    pub fn new() -> EpochQuery {
        EpochQuery {
            start_epoch: None,
            count: None,
            ascending: true,
        }
    }

    #[wasm_bindgen(setter, js_name = setStartEpoch)]
    pub fn set_start_epoch(&mut self, epoch: u32) {
        self.start_epoch = Some(epoch);
    }

    #[wasm_bindgen(setter)]
    pub fn set_count(&mut self, count: u32) {
        self.count = Some(count);
    }

    #[wasm_bindgen(setter)]
    pub fn set_ascending(&mut self, ascending: bool) {
        self.ascending = ascending;
    }

    #[wasm_bindgen(getter, js_name = startEpoch)]
    pub fn start_epoch(&self) -> Option<u32> {
        self.start_epoch
    }

    #[wasm_bindgen(getter)]
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    #[wasm_bindgen(getter)]
    pub fn ascending(&self) -> bool {
        self.ascending
    }
}

/// Query for contested resources (voting)
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ContestedResourceQuery {
    contract_id: Identifier,
    document_type: String,
    index_name: String,
    start_value: Option<Vec<u8>>,
    start_included: bool,
    limit: Option<u32>,
}

#[wasm_bindgen]
impl ContestedResourceQuery {
    #[wasm_bindgen(constructor)]
    pub fn new(
        contract_id: &str,
        document_type: &str,
        index_name: &str,
    ) -> Result<ContestedResourceQuery, JsError> {
        let contract_id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract identifier: {}", e)))?;

        Ok(ContestedResourceQuery {
            contract_id,
            document_type: document_type.to_string(),
            index_name: index_name.to_string(),
            start_value: None,
            start_included: false,
            limit: None,
        })
    }

    #[wasm_bindgen(setter, js_name = setStartValue)]
    pub fn set_start_value(&mut self, value: Vec<u8>) {
        self.start_value = Some(value);
    }

    #[wasm_bindgen(setter, js_name = setStartIncluded)]
    pub fn set_start_included(&mut self, included: bool) {
        self.start_included = included;
    }

    #[wasm_bindgen(setter)]
    pub fn set_limit(&mut self, limit: u32) {
        self.limit = Some(limit);
    }
}

impl ContestedResourceQuery {
    pub fn contract_identifier(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_name(&self) -> &str {
        &self.index_name
    }

    pub fn start_value(&self) -> Option<&[u8]> {
        self.start_value.as_deref()
    }

    pub fn limit(&self) -> Option<u32> {
        self.limit
    }
}

/// Simple Drive query representation for WASM
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleDriveQuery {
    pub contract_id: Identifier,
    pub document_type: String,
    pub where_clauses: Vec<WhereClause>,
    pub order_by: Vec<OrderByClause>,
    pub limit: Option<u32>,
    pub start_at: Option<Vec<u8>>,
    pub start_after: Option<Vec<u8>>,
}

/// Build a Drive query from JavaScript parameters
pub fn build_drive_query(
    contract_id: &str,
    document_type: &str,
    where_clause: JsValue,
    order_by: JsValue,
    limit: Option<u32>,
    start_at: Option<Vec<u8>>,
    start_after: Option<Vec<u8>>,
) -> Result<SimpleDriveQuery, JsError> {
    let contract_id = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let mut where_clauses = Vec::new();
    let mut order_by_clauses = Vec::new();

    // Parse where clause
    if !where_clause.is_null() && !where_clause.is_undefined() {
        if let Some(where_obj) = where_clause.dyn_ref::<Object>() {
            let entries = Object::entries(where_obj);
            for i in 0..entries.length() {
                let entry = entries.get(i);
                if let Some(entry_array) = entry.dyn_ref::<Array>() {
                    if entry_array.length() >= 2 {
                        let field = entry_array
                            .get(0)
                            .as_string()
                            .ok_or_else(|| JsError::new("Field name must be a string"))?;
                        let value = entry_array.get(1);

                        // For simple equality checks
                        where_clauses.push(WhereClause {
                            field,
                            operator: WhereOperator::Equal,
                            value: serde_wasm_bindgen::from_value(value).map_err(|e| {
                                JsError::new(&format!("Invalid where value: {}", e))
                            })?,
                        });
                    }
                }
            }
        }
    }

    // Parse order by
    if !order_by.is_null() && !order_by.is_undefined() {
        if let Some(order_array) = order_by.dyn_ref::<Array>() {
            for i in 0..order_array.length() {
                let order_item = order_array.get(i);
                if let Some(order_obj) = order_item.dyn_ref::<Object>() {
                    let field = Reflect::get(order_obj, &"field".into())
                        .ok()
                        .and_then(|v| v.as_string())
                        .ok_or_else(|| JsError::new("Order field must be a string"))?;

                    let ascending = Reflect::get(order_obj, &"ascending".into())
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);

                    order_by_clauses.push(OrderByClause { field, ascending });
                }
            }
        }
    }

    Ok(SimpleDriveQuery {
        contract_id,
        document_type: document_type.to_string(),
        where_clauses,
        order_by: order_by_clauses,
        limit,
        start_at,
        start_after,
    })
}
