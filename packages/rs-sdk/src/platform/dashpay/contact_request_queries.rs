//! Contact request query helpers
//!
//! This module provides helper functions for querying contact requests from the platform.
//!
//! The fetch is **incremental and fully paginated** (see
//! `docs/dashpay/SYNC_CORRECTNESS_SPEC.md`): an optional `after_created_at`
//! lower bound restricts the query to documents newer than the caller's
//! high-water mark, and the helper drains *all* pages via a `StartAfter`
//! document-id cursor so a flood of requests can never bury (truncate) the
//! newest ones. Returning `Ok` means pagination ran to exhaustion without
//! error — the caller may then advance its high-water cursor; any page error
//! propagates as `Err`, leaving the caller's cursor untouched.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::document::Document;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::platform_value::platform_value;
use dpp::prelude::Identifier;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;

/// Result of a contact request query containing the parsed documents
pub type ContactRequestDocuments = Documents;

/// Page size for the paginated contact-request fetch. The fetch drains every
/// page (retrieve-all); this only bounds how many documents move per round
/// trip. 100 is the platform document-query maximum.
const CONTACT_REQUEST_PAGE_SIZE: u32 = 100;

impl Sdk {
    /// Drain every `contactRequest` document matching `filter_field ==
    /// identity_id` (and, if `after_created_at` is set, `$createdAt >
    /// after_created_at`), paginating with a `StartAfter` document-id cursor
    /// until a short/empty page proves exhaustion.
    ///
    /// `Ok` ⇒ all pages fetched (the caller may advance its high-water mark);
    /// any page error short-circuits as `Err` so the caller does not advance.
    async fn fetch_contact_requests_paginated(
        &self,
        filter_field: &str,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        let mut where_clauses = vec![WhereClause {
            field: filter_field.to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(identity_id),
        }];
        if let Some(after) = after_created_at {
            where_clauses.push(WhereClause {
                field: "$createdAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!(after),
            });
        }

        let mut all: ContactRequestDocuments = Default::default();
        let mut start: Option<Start> = None;

        loop {
            let query = DocumentQuery {
                select: drive::query::SelectProjection::documents(),
                data_contract: dashpay_contract.clone(),
                document_type_name: "contactRequest".to_string(),
                where_clauses: where_clauses.clone(),
                group_by: vec![],
                having: vec![],
                // Load-bearing: a bare secondary-index equality with no
                // order-by is silently proven ABSENT by drive (observed
                // against drive 4.0.0-rc.2: `toUserId ==` returned a verified
                // empty result for an existing document). The clause also
                // pins the query to the contract's `(field, $createdAt)`
                // index, giving the deterministic order pagination relies on.
                order_by_clauses: vec![OrderClause {
                    field: "$createdAt".to_string(),
                    ascending: true,
                }],
                limit: CONTACT_REQUEST_PAGE_SIZE,
                start: start.clone(),
            };

            let page = Document::fetch_many(self, query).await?;
            let page_len = page.len();
            // The last document id in query order seeds the next page's
            // cursor (distinct from the `$createdAt` high-water the caller
            // tracks — this id cursor is ephemeral, per-loop).
            let last_id = page.keys().last().copied();
            for (id, doc) in page {
                all.insert(id, doc);
            }

            // A short page proves exhaustion (a full page may have more).
            if page_len < CONTACT_REQUEST_PAGE_SIZE as usize {
                break;
            }
            match last_id {
                Some(id) => start = Some(Start::StartAfter(id.to_buffer().to_vec())),
                None => break,
            }
        }

        Ok(all)
    }

    /// Fetch contact requests **sent** by `identity_id` (`$ownerId ==`),
    /// newer than `after_created_at` if given, fully paginated.
    pub async fn fetch_sent_contact_requests(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        self.fetch_contact_requests_paginated("$ownerId", identity_id, after_created_at)
            .await
    }

    /// Fetch contact requests **received** by `identity_id` (`toUserId ==`),
    /// newer than `after_created_at` if given, fully paginated.
    pub async fn fetch_received_contact_requests(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        self.fetch_contact_requests_paginated("toUserId", identity_id, after_created_at)
            .await
    }

    /// Fetch both sent and received contact requests for an identity, each
    /// newer than `after_created_at` if given.
    pub async fn fetch_all_contact_requests_for_identity(
        &self,
        identity: &Identity,
        after_created_at: Option<u64>,
    ) -> Result<(ContactRequestDocuments, ContactRequestDocuments), Error> {
        let identity_id = identity.id();

        let (sent_result, received_result) = tokio::join!(
            self.fetch_sent_contact_requests(identity_id, after_created_at),
            self.fetch_received_contact_requests(identity_id, after_created_at)
        );

        Ok((sent_result?, received_result?))
    }
}
