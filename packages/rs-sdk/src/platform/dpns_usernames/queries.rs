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
}

impl Sdk {
    /// Get DPNS usernames owned by a specific identity
    ///
    /// This searches for domains where the identity is listed in records.identity.
    /// Note: This does not search for domains owned by the identity (no index on $ownerId)
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

        // Query for domains with this identity in records.identity (the only indexed identity field)
        let records_identity_query = DocumentQuery {
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            order_by_clauses: vec![], // Remove ordering by $createdAt as it might not be indexed
            limit,
            start: None,
        };

        let records_identity_documents = Document::fetch_many(self, records_identity_query).await?;

        let mut usernames = Vec::new();
        for (_, doc_opt) in records_identity_documents {
            if let Some(doc) = doc_opt {
                if let Some(username) = Self::document_to_dpns_username(doc) {
                    usernames.push(username);
                }
            }
        }

        Ok(usernames)
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
    pub async fn resolve_dpns_name_to_identity(
        &self,
        name: &str,
    ) -> Result<Option<Identifier>, Error> {
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

        // Extract identity ID from records if present
        let records_identity_id = if let Some(Value::Map(records)) = properties.get("records") {
            // Look for the "identity" key in the map
            records
                .iter()
                .find(|(k, _)| k.as_text() == Some("identity"))
                .and_then(|(_, v)| v.to_identifier().ok())
        } else {
            None
        };

        Some(DpnsUsername {
            label: label.clone(),
            normalized_label,
            full_name: format!("{}.{}", label, parent_domain),
            owner_id: doc.owner_id(),
            records_identity_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::SdkBuilder;
    use dpp::dashcore::Network;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore] // Requires network connection
    async fn test_dpns_queries() {
        use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
        use std::num::NonZeroUsize;

        // Create trusted context provider for testnet
        let context_provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,                            // No devnet name
            NonZeroUsize::new(100).unwrap(), // Cache size
        )
        .expect("Failed to create context provider");

        // Create SDK with testnet configuration and trusted context provider
        let address_list = "https://52.12.176.90:1443"
            .parse()
            .expect("Failed to parse address");
        let sdk = SdkBuilder::new(address_list)
            .with_network(Network::Testnet)
            .with_context_provider(context_provider)
            .build()
            .expect("Failed to create SDK");

        // Test search
        let results = sdk.search_dpns_names("test", Some(5)).await.unwrap();
        println!("Search results for 'test': {:?}", results);

        // Test availability check
        let is_available = sdk
            .check_dpns_name_availability("somerandomunusedname123456")
            .await
            .unwrap();
        assert!(is_available, "Random name should be available");

        // Test resolve (if we know a name exists)
        if let Ok(Some(identity_id)) = sdk
            .resolve_dpns_name_to_identity("therealslimshaddy5")
            .await
        {
            println!("'therealslimshaddy5' resolves to identity: {}", identity_id);

            // Test get usernames by identity
            let usernames = sdk
                .get_dpns_usernames_by_identity(identity_id, Some(5))
                .await
                .unwrap();
            println!("Usernames for identity {}: {:?}", identity_id, usernames);
        }
    }
}
