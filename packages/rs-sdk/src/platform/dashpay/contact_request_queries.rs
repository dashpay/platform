//! Contact request query helpers
//!
//! This module provides helper functions for querying contact requests from the platform.
//!
//! The fetch is **incremental and paginated**: an optional `after_created_at`
//! lower bound restricts the query to documents newer than the caller's
//! high-water mark, and the helper continues page after page with a `startAt`
//! document-id cursor until a short page proves exhaustion or the per-sweep
//! page budget is reached. Returning `Ok` means no page failed — the caller
//! may then advance its high-water cursor; any page error propagates as
//! `Err`, leaving the caller's cursor untouched. A budget stop can end inside
//! a block, and `after_created_at` is exclusive, so a caller must resume
//! below the max `$createdAt` it fetched (the wallet rewinds 10 minutes),
//! never from it.

use std::sync::Arc;

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::platform_value::platform_value;
use dpp::prelude::Identifier;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;

/// Result of a contact request query containing the parsed documents
pub type ContactRequestDocuments = Documents;

/// Page size for the paginated contact-request fetch. The fetch drains pages
/// (retrieve-all, up to the per-sweep budget); this only bounds how many
/// documents move per round trip. 100 is the platform document-query maximum.
const CONTACT_REQUEST_PAGE_SIZE: u32 = 100;

/// Per-sweep page budget. `contactRequest` documents are public and freely
/// indexable by `toUserId`, so a hostile sender can flood a target with cheap
/// throwaway requests; without a cap, every cold-start / restore sweep would
/// fetch and hold the entire spam set in memory at once. The fetch is
/// `$createdAt`-ascending and the caller resumes below the max `$createdAt`
/// fetched, so capping pages spreads a large backlog across sweeps
/// oldest-first. A continuation page repeats its cursor row, so one
/// sweep returns at most 100 + 49 × 99 = 4_951 documents, far above any
/// legitimate pending-request count (and a legit user above it just takes an
/// extra sweep to fully ingest).
///
/// Within a sweep every request is returned once, including the requests of
/// a block that a page boundary cuts (see [`contact_request_page_query`]).
/// Between sweeps, forward progress assumes no window the caller resumes
/// from holds ≥ this budget of matching documents: the wallet caller rewinds
/// each sweep's lower `$createdAt` bound by a 10-minute overlap
/// (`SYNC_OVERLAP_MS`), so an attacker who concentrates ~5_000 fee-paid
/// `contactRequest`s within any 10-minute span targeting one recipient keeps
/// the high-water pinned inside that window and the next sweep re-reads the
/// same oldest documents. The memory bound still holds (oldest-first,
/// budget-capped); only forward progress past such a saturated window
/// stalls. The recovery is to resume a sweep from a persisted `startAt`
/// document-id cursor instead of a `$createdAt` high-water.
const MAX_CONTACT_REQUEST_PAGES_PER_SWEEP: u32 = 50;

/// One page of a contact-request sweep, in query order.
struct ContactRequestPage {
    /// The page's documents, without the cursor row of a continuation.
    documents: ContactRequestDocuments,
    /// How many rows the proof returned, the cursor row included: a page
    /// that fills [`CONTACT_REQUEST_PAGE_SIZE`] may have more after it.
    raw_len: usize,
    /// The page's last row, where the next page starts.
    last_id: Option<Identifier>,
}

/// The query for one page of `contactRequest` documents matching
/// `filter_field == identity_id`, oldest first, starting at `cursor` (the
/// last row of the previous page) when continuing.
///
/// Requests created in one block share their `$createdAt`. With `$createdAt`
/// only in the order, Drive bounds a `startAt` cursor's own `$createdAt` by
/// the cursor's document id, so a continuation resumes inside the cursor's
/// block at every protocol version. Up to protocol version 13 Drive does not
/// apply that bound to `startAfter` (it skips the rest of the cursor's block)
/// or to a cursor combined with a `$createdAt` clause (it skips or re-reads
/// the block). So only a first page carries `$createdAt > after_created_at`;
/// every row after a cursor that such a first page returned satisfies it
/// anyway.
fn contact_request_page_query(
    data_contract: Arc<DataContract>,
    filter_field: &str,
    identity_id: Identifier,
    after_created_at: Option<u64>,
    cursor: Option<Identifier>,
) -> DocumentQuery {
    let mut where_clauses = vec![WhereClause {
        field: filter_field.to_string(),
        operator: WhereOperator::Equal,
        value: platform_value!(identity_id),
    }];
    if let (None, Some(after)) = (cursor, after_created_at) {
        where_clauses.push(WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!(after),
        });
    }
    DocumentQuery {
        select: drive::query::SelectProjection::documents(),
        data_contract,
        document_type_name: "contactRequest".to_string(),
        where_clauses,
        time_range_clauses: vec![],
        integer_range_clauses: vec![],
        sub_queries: vec![],
        group_by: vec![],
        having: vec![],
        // Load-bearing: a bare secondary-index equality with no order-by is
        // silently proven ABSENT by drive (observed against drive
        // 4.0.0-rc.2: `toUserId ==` returned a verified empty result for an
        // existing document). The clause also pins the query to the
        // contract's `(field, $createdAt)` index, giving the deterministic
        // order pagination relies on.
        order_by_clauses: vec![OrderClause {
            field: "$createdAt".to_string(),
            ascending: true,
        }],
        limit: CONTACT_REQUEST_PAGE_SIZE,
        offset: None,
        start: cursor.map(|id| Start::StartAt(id.to_buffer().to_vec())),
    }
}

impl Sdk {
    /// Fetch one page of [`contact_request_page_query`]. A continuation's
    /// proof returns the cursor document as its first row; it is checked and
    /// dropped. A first row that is not the cursor, or (with
    /// `after_created_at`) a cursor not created after it, means the cursor
    /// did not come from this query, and the page is refused.
    async fn fetch_contact_request_page(
        &self,
        data_contract: &Arc<DataContract>,
        filter_field: &str,
        identity_id: Identifier,
        after_created_at: Option<u64>,
        cursor: Option<Identifier>,
    ) -> Result<ContactRequestPage, Error> {
        let query = contact_request_page_query(
            Arc::clone(data_contract),
            filter_field,
            identity_id,
            after_created_at,
            cursor,
        );
        let page = Document::fetch_many(self, query).await?;
        let raw_len = page.len();
        // Relies on `Documents` being insertion-ordered (`IndexMap`), so the
        // last key is the `$createdAt`-ascending last row; a `BTreeMap` here
        // would silently reorder by doc id and break pagination.
        let last_id = page.keys().last().copied();
        let mut rows = page.into_iter();
        if let Some(cursor) = cursor {
            let cursor_created_at = match rows.next() {
                Some((id, Some(document))) if id == cursor => document.created_at(),
                _ => {
                    return Err(Error::InvalidProvedResponse(format!(
                        "the contact-request page of {filter_field} == {identity_id} at \
                         cursor {cursor} does not start with the cursor"
                    )))
                }
            };
            if let Some(after) = after_created_at {
                if !cursor_created_at.is_some_and(|created_at| created_at > after) {
                    return Err(Error::InvalidProvedResponse(format!(
                        "the contact-request cursor {cursor} was created at \
                         {cursor_created_at:?}, not after {after}, so no first page \
                         of this query returned it"
                    )));
                }
            }
        }
        Ok(ContactRequestPage {
            documents: rows.collect(),
            raw_len,
            last_id,
        })
    }

    /// Drain `contactRequest` documents matching `filter_field ==
    /// identity_id` (and, if `after_created_at` is set, `$createdAt >
    /// after_created_at`), paginating with a `startAt` document-id cursor
    /// until a short/empty page proves exhaustion **or** the per-sweep page
    /// budget ([`MAX_CONTACT_REQUEST_PAGES_PER_SWEEP`]) is hit.
    ///
    /// `Ok` ⇒ the fetch completed without error and the caller may advance its
    /// high-water mark to the max `$createdAt` fetched; a page error
    /// short-circuits as `Err` so the caller does not advance. The result may
    /// be a budgeted PARTIAL (oldest-first) under a `toUserId` flood, cut
    /// inside a block, so the next sweep must start below that high-water
    /// mark to read the remainder.
    async fn fetch_contact_requests_paginated(
        &self,
        filter_field: &str,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        let mut all: ContactRequestDocuments = Default::default();
        let mut cursor: Option<Identifier> = None;
        let mut pages_fetched: u32 = 0;

        loop {
            let page = self
                .fetch_contact_request_page(
                    &dashpay_contract,
                    filter_field,
                    identity_id,
                    after_created_at,
                    cursor,
                )
                .await?;
            all.extend(page.documents);
            pages_fetched += 1;

            // Stop on a short page (exhaustion) or the per-sweep page budget. A
            // budget stop on a still-full page is logged: `all` holds the oldest
            // requests (`$createdAt ASC`), and the caller's next sweep resumes
            // below the max `$createdAt` fetched.
            if !should_fetch_another_contact_request_page(page.raw_len, pages_fetched) {
                if page.raw_len >= CONTACT_REQUEST_PAGE_SIZE as usize {
                    tracing::warn!(
                        filter_field,
                        documents = all.len(),
                        pages = pages_fetched,
                        "contact-request sweep hit the per-sweep page budget; \
                         resuming the remainder next sweep"
                    );
                }
                break;
            }
            match page.last_id {
                Some(id) => cursor = Some(id),
                None => break,
            }
        }

        Ok(all)
    }

    /// Fetch contact requests **sent** by `identity_id` (`$ownerId ==`),
    /// newer than `after_created_at` if given, paginated up to the per-sweep
    /// page budget.
    pub async fn fetch_sent_contact_requests(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        self.fetch_contact_requests_paginated("$ownerId", identity_id, after_created_at)
            .await
    }

    /// Fetch contact requests **received** by `identity_id` (`toUserId ==`),
    /// newer than `after_created_at` if given, paginated up to the per-sweep
    /// page budget.
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

/// Whether the contact-request pagination loop should fetch another page:
/// only when the last page was full (more may remain) AND the per-sweep page
/// budget has not been reached. A short page (exhaustion) stops first and
/// takes priority over the budget.
fn should_fetch_another_contact_request_page(last_page_len: usize, pages_fetched: u32) -> bool {
    last_page_len >= CONTACT_REQUEST_PAGE_SIZE as usize
        && pages_fetched < MAX_CONTACT_REQUEST_PAGES_PER_SWEEP
}

#[cfg(all(test, feature = "mocks", not(target_arch = "wasm32")))]
mod paging_tests;

#[cfg(test)]
mod tests {
    use super::{
        should_fetch_another_contact_request_page, CONTACT_REQUEST_PAGE_SIZE,
        MAX_CONTACT_REQUEST_PAGES_PER_SWEEP,
    };

    #[test]
    fn pagination_continues_until_budget_then_stops() {
        let full = CONTACT_REQUEST_PAGE_SIZE as usize;
        // Full page, still under budget → keep draining.
        assert!(should_fetch_another_contact_request_page(full, 1));
        assert!(should_fetch_another_contact_request_page(
            full,
            MAX_CONTACT_REQUEST_PAGES_PER_SWEEP - 1
        ));
        // Full page, budget reached → stop (the high-water cursor resumes the
        // remainder next sweep; a spam flood can't force an unbounded fetch).
        assert!(!should_fetch_another_contact_request_page(
            full,
            MAX_CONTACT_REQUEST_PAGES_PER_SWEEP
        ));
        // A short page is exhaustion — stop regardless of how few pages ran.
        assert!(!should_fetch_another_contact_request_page(full - 1, 1));
        assert!(!should_fetch_another_contact_request_page(0, 1));
    }
}
