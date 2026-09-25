use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::{Document, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
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
            select: drive::query::SelectProjection::documents(),
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![WhereClause {
                field: "records.identity".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(identity_id.to_buffer()),
            }],
            time_range_clauses: vec![],
            sub_queries: vec![],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![], // Remove ordering by $createdAt as it might not be indexed
            limit,
            offset: None,
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

    /// Get every DPNS username associated with `identity_id`, paging through
    /// the `records.identity` index `page_limit` documents at a time.
    ///
    /// Returns the usernames and whether the set is known to be complete:
    /// `true` once a page comes back shorter than `page_limit`, `false` when
    /// `max_pages` full pages were read without reaching the end (the caller
    /// must then treat the result as a lower bound, not the owned set).
    /// Documents that do not parse as DPNS domains are skipped, as in
    /// [`Self::get_dpns_usernames_by_identity`].
    pub async fn get_all_dpns_usernames_by_identity(
        &self,
        identity_id: Identifier,
        page_limit: u32,
        max_pages: usize,
    ) -> Result<(Vec<DpnsUsername>, bool), Error> {
        let dpns_contract = self.fetch_dpns_contract().await?;
        let mut usernames = Vec::new();
        let mut start_after: Option<Identifier> = None;
        for _ in 0..max_pages {
            let query = DocumentQuery {
                select: drive::query::SelectProjection::documents(),
                data_contract: dpns_contract.clone(),
                document_type_name: "domain".to_string(),
                where_clauses: vec![WhereClause {
                    field: "records.identity".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(identity_id.to_buffer()),
                }],
                time_range_clauses: vec![],
                sub_queries: vec![],
                group_by: vec![],
                having: vec![],
                order_by_clauses: vec![],
                limit: page_limit,
                offset: None,
                start: start_after.map(|id| Start::StartAfter(id.to_vec())),
            };
            let documents = Document::fetch_many(self, query).await?;
            let page_len = documents.len();
            let last_id = documents.keys().last().copied();
            usernames.extend(
                documents
                    .into_values()
                    .flatten()
                    .filter_map(Self::document_to_dpns_username),
            );
            match next_dpns_page(page_len, page_limit, last_id) {
                DpnsPageStep::Complete => return Ok((usernames, true)),
                DpnsPageStep::Continue(cursor) => start_after = Some(cursor),
                DpnsPageStep::Unknown => return Ok((usernames, false)),
            }
        }
        Ok((usernames, false))
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
            select: drive::query::SelectProjection::documents(),
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
            time_range_clauses: vec![],
            sub_queries: vec![],
            group_by: vec![],
            having: vec![],
            order_by_clauses: vec![OrderClause {
                field: "normalizedLabel".to_string(),
                ascending: true,
            }],
            limit: limit.unwrap_or(10),
            offset: None,
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

/// What to do after one page of a paged DPNS username query.
#[derive(Debug, PartialEq, Eq)]
enum DpnsPageStep {
    /// A short page: the whole set has been read.
    Complete,
    /// A full page: continue after this document id.
    Continue(Identifier),
    /// A full page without a cursor: completeness cannot be established.
    Unknown,
}

fn next_dpns_page(page_len: usize, page_limit: u32, last_id: Option<Identifier>) -> DpnsPageStep {
    if page_len < page_limit as usize {
        DpnsPageStep::Complete
    } else if let Some(id) = last_id {
        DpnsPageStep::Continue(id)
    } else {
        DpnsPageStep::Unknown
    }
}

#[cfg(test)]
mod paging_tests {
    use super::*;

    #[test]
    fn a_short_page_completes_the_set() {
        assert_eq!(next_dpns_page(0, 100, None), DpnsPageStep::Complete);
        assert_eq!(
            next_dpns_page(99, 100, Some(Identifier::from([1u8; 32]))),
            DpnsPageStep::Complete
        );
    }

    #[test]
    fn a_full_page_continues_after_its_last_document() {
        let last = Identifier::from([7u8; 32]);
        assert_eq!(
            next_dpns_page(100, 100, Some(last)),
            DpnsPageStep::Continue(last)
        );
    }

    #[test]
    fn a_full_page_without_a_cursor_is_not_complete() {
        assert_eq!(next_dpns_page(100, 100, None), DpnsPageStep::Unknown);
    }
}
