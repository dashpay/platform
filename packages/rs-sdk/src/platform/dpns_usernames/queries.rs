use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::{Document, FetchMany};
use crate::{Error, Sdk};
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use drive::query::{OrderClause, WhereClause, WhereOperator};

use super::convert_to_homograph_safe_chars;

/// Result of a DPNS username search
#[derive(Debug, Clone)]
pub struct DpnsUsername {
    /// The domain label (e.g., "alice")
    pub label: String,
    /// The normalized label (e.g., "a11ce") 
    pub normalized_label: String,
    /// The full domain name (e.g., "alice.dash")
    pub full_name: String,
    /// The identity ID that owns this domain
    pub owner_id: Identifier,
    /// The identity ID from the records (may be different from owner)
    pub records_identity_id: Option<Identifier>,
    /// The alias identity ID from the records
    pub records_alias_identity_id: Option<Identifier>,
}

impl Sdk {
    /// Get DPNS usernames owned by a specific identity
    ///
    /// This searches for domains where the identity is either:
    /// - The owner of the domain document
    /// - Listed in records.dashUniqueIdentityId
    /// - Listed in records.dashAliasIdentityId
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID to search for
    /// * `limit` - Maximum number of results to return (default: 10)
    ///
    /// # Returns
    ///
    /// Returns a list of DPNS usernames associated with the identity
    pub async fn get_dpns_usernames_by_identity(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<Vec<DpnsUsername>, Error> {
        let dpns_contract = self.fetch_dpns_contract().await?;
        let limit = limit.unwrap_or(10);
        let mut all_usernames = Vec::new();

        // Query 1: Check for domains owned by this identity
        let owner_query = DocumentQuery {
            data_contract: dpns_contract.clone(),
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
            ],
            order_by_clauses: vec![OrderClause {
                field: "$createdAt".to_string(),
                ascending: false,
            }],
            limit,
            start: None,
        };

        let owner_documents = Document::fetch_many(self, owner_query).await?;
        
        // Filter by owner_id and convert to DpnsUsername
        for (_, doc_opt) in owner_documents {
            if let Some(doc) = doc_opt {
                if doc.owner_id() == identity_id {
                    if let Some(username) = Self::document_to_dpns_username(doc) {
                        all_usernames.push(username);
                        if all_usernames.len() >= limit as usize {
                            return Ok(all_usernames);
                        }
                    }
                }
            }
        }

        // Query 2: Check for domains with this identity in records.dashUniqueIdentityId
        let unique_id_query = DocumentQuery {
            data_contract: dpns_contract.clone(),
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "records.dashUniqueIdentityId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(identity_id.to_buffer()),
                },
            ],
            order_by_clauses: vec![OrderClause {
                field: "$createdAt".to_string(),
                ascending: false,
            }],
            limit: limit - all_usernames.len() as u32,
            start: None,
        };

        let unique_id_documents = Document::fetch_many(self, unique_id_query).await?;
        
        for (_, doc_opt) in unique_id_documents {
            if let Some(doc) = doc_opt {
                if let Some(username) = Self::document_to_dpns_username(doc) {
                    // Avoid duplicates
                    if !all_usernames.iter().any(|u| u.normalized_label == username.normalized_label) {
                        all_usernames.push(username);
                        if all_usernames.len() >= limit as usize {
                            return Ok(all_usernames);
                        }
                    }
                }
            }
        }

        // Query 3: Check for domains with this identity in records.dashAliasIdentityId
        if all_usernames.len() < limit as usize {
            let alias_id_query = DocumentQuery {
                data_contract: dpns_contract,
                document_type_name: "domain".to_string(),
                where_clauses: vec![
                    WhereClause {
                        field: "normalizedParentDomainName".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("dash".to_string()),
                    },
                    WhereClause {
                        field: "records.dashAliasIdentityId".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Identifier(identity_id.to_buffer()),
                    },
                ],
                order_by_clauses: vec![OrderClause {
                    field: "$createdAt".to_string(),
                    ascending: false,
                }],
                limit: limit - all_usernames.len() as u32,
                start: None,
            };

            let alias_id_documents = Document::fetch_many(self, alias_id_query).await?;
            
            for (_, doc_opt) in alias_id_documents {
                if let Some(doc) = doc_opt {
                    if let Some(username) = Self::document_to_dpns_username(doc) {
                        // Avoid duplicates
                        if !all_usernames.iter().any(|u| u.normalized_label == username.normalized_label) {
                            all_usernames.push(username);
                            if all_usernames.len() >= limit as usize {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(all_usernames)
    }

    /// Check if a DPNS username is available
    ///
    /// # Arguments
    ///
    /// * `label` - The username label to check (e.g., "alice")
    ///
    /// # Returns
    ///
    /// Returns `true` if the username is available, `false` if it's taken
    pub async fn check_dpns_name_availability(&self, label: &str) -> Result<bool, Error> {
        // Use the existing method from mod.rs
        self.is_dpns_name_available(label).await
    }

    /// Resolve a DPNS name to an identity ID
    ///
    /// # Arguments
    ///
    /// * `name` - The full domain name (e.g., "alice.dash") or just the label (e.g., "alice")
    ///
    /// # Returns
    ///
    /// Returns the identity ID associated with the domain, or None if not found
    pub async fn resolve_dpns_name_to_identity(&self, name: &str) -> Result<Option<Identifier>, Error> {
        // Use the existing method from mod.rs
        self.resolve_dpns_name(name).await
    }

    /// Search for DPNS names that start with a given prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to search for (e.g., "ali" to find "alice", "alicia", etc.)
    /// * `limit` - Maximum number of results to return (default: 10)
    ///
    /// # Returns
    ///
    /// Returns a list of DPNS usernames that match the prefix
    pub async fn search_dpns_names(
        &self,
        prefix: &str,
        limit: Option<u32>,
    ) -> Result<Vec<DpnsUsername>, Error> {
        let dpns_contract = self.fetch_dpns_contract().await?;
        let normalized_prefix = convert_to_homograph_safe_chars(prefix);

        let query = DocumentQuery {
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::StartsWith,
                    value: Value::Text(normalized_prefix),
                },
            ],
            order_by_clauses: vec![OrderClause {
                field: "normalizedLabel".to_string(),
                ascending: true,
            }],
            limit: limit.unwrap_or(10),
            start: None,
        };

        let documents = Document::fetch_many(self, query).await?;
        let mut usernames = Vec::new();

        for (_, doc_opt) in documents {
            if let Some(doc) = doc_opt {
                if let Some(username) = Self::document_to_dpns_username(doc) {
                    usernames.push(username);
                }
            }
        }

        Ok(usernames)
    }

    /// Helper function to convert a DPNS domain document to DpnsUsername struct
    fn document_to_dpns_username(doc: Document) -> Option<DpnsUsername> {
        let properties = doc.properties();
        
        let label = properties.get("label")?.as_text()?.to_string();
        let normalized_label = properties.get("normalizedLabel")?.as_text()?.to_string();
        let parent_domain = properties.get("normalizedParentDomainName")?.as_text()?;
        
        // Extract identity IDs from records if present
        let (records_identity_id, records_alias_identity_id) = if let Some(Value::Map(records)) = properties.get("records") {
            let mut unique_id = None;
            let mut alias_id = None;
            
            for (key, value) in records {
                if let (Value::Text(k), Value::Identifier(id_bytes)) = (key, value) {
                    match k.as_str() {
                        "dashUniqueIdentityId" => {
                            unique_id = Identifier::from_bytes(id_bytes).ok();
                        }
                        "dashAliasIdentityId" => {
                            alias_id = Identifier::from_bytes(id_bytes).ok();
                        }
                        _ => {}
                    }
                }
            }
            
            (unique_id, alias_id)
        } else {
            (None, None)
        };

        Some(DpnsUsername {
            label: label.clone(),
            normalized_label,
            full_name: format!("{}.{}", label, parent_domain),
            owner_id: doc.owner_id(),
            records_identity_id,
            records_alias_identity_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_dpns_queries() {
        let sdk = crate::Sdk::builder()
            .build()
            .await
            .expect("Failed to create SDK");

        // Test search
        let results = sdk.search_dpns_names("test", Some(5)).await.unwrap();
        println!("Search results for 'test': {:?}", results);

        // Test availability check
        let is_available = sdk.check_dpns_name_availability("somerandomunusedname123456").await.unwrap();
        assert!(is_available, "Random name should be available");

        // Test resolve (if we know a name exists)
        if let Ok(Some(identity_id)) = sdk.resolve_dpns_name_to_identity("dash").await {
            println!("'dash' resolves to identity: {}", identity_id);
            
            // Test get usernames by identity
            let usernames = sdk.get_dpns_usernames_by_identity(identity_id, Some(5)).await.unwrap();
            println!("Usernames for identity {}: {:?}", identity_id, usernames);
        }
    }
}