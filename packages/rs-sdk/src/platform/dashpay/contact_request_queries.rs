//! Contact request query helpers
//!
//! This module provides helper functions for querying contact requests from the platform

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::document::Document;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::platform_value::platform_value;
use dpp::prelude::Identifier;
use drive::query::{WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;

/// Result of a contact request query containing the parsed documents
pub type ContactRequestDocuments = Documents;

impl Sdk {
    /// Fetch all contact requests sent by a specific identity
    ///
    /// This queries the DashPay contract for contactRequest documents where
    /// the given identity is the owner (sender).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID of the sender
    /// * `limit` - Maximum number of contact requests to fetch (default: 100)
    ///
    /// # Returns
    ///
    /// Returns a map of document IDs to optional contact request documents
    pub async fn fetch_sent_contact_requests(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<ContactRequestDocuments, Error> {
        // Fetch the DashPay contract
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        // Query for sent contact requests (where this identity is the owner)
        // Note: We need to filter by $ownerId to get only this identity's sent requests
        let query = DocumentQuery {
            data_contract: dashpay_contract,
            document_type_name: "contactRequest".to_string(),
            where_clauses: vec![WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            order_by_clauses: vec![],
            limit: limit.unwrap_or(100),
            start: None,
        };

        // Fetch the documents
        Document::fetch_many(self, query).await
    }

    /// Fetch all contact requests received by a specific identity
    ///
    /// This queries the DashPay contract for contactRequest documents where
    /// the given identity is the recipient (toUserId field).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity ID of the recipient
    /// * `limit` - Maximum number of contact requests to fetch (default: 100)
    ///
    /// # Returns
    ///
    /// Returns a map of document IDs to optional contact request documents
    pub async fn fetch_received_contact_requests(
        &self,
        identity_id: Identifier,
        limit: Option<u32>,
    ) -> Result<ContactRequestDocuments, Error> {
        // Fetch the DashPay contract
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        // Query for received contact requests (where this identity is toUserId)
        let query = DocumentQuery {
            data_contract: dashpay_contract,
            document_type_name: "contactRequest".to_string(),
            where_clauses: vec![WhereClause {
                field: "toUserId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            order_by_clauses: vec![],
            limit: limit.unwrap_or(100),
            start: None,
        };

        // Fetch the documents
        Document::fetch_many(self, query).await
    }

    /// Fetch all contact requests for a specific identity (both sent and received)
    ///
    /// This is a convenience method that fetches both sent and received contact requests
    /// for a given identity.
    ///
    /// # Arguments
    ///
    /// * `identity` - The identity to fetch contact requests for
    /// * `limit` - Maximum number of contact requests to fetch per query (default: 100)
    ///
    /// # Returns
    ///
    /// Returns a tuple of (sent_requests, received_requests)
    pub async fn fetch_all_contact_requests_for_identity(
        &self,
        identity: &Identity,
        limit: Option<u32>,
    ) -> Result<(ContactRequestDocuments, ContactRequestDocuments), Error> {
        let identity_id = identity.id();

        // Fetch both sent and received contact requests in parallel
        let (sent_result, received_result) = tokio::join!(
            self.fetch_sent_contact_requests(identity_id, limit),
            self.fetch_received_contact_requests(identity_id, limit)
        );

        Ok((sent_result?, received_result?))
    }
}
