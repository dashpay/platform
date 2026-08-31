//! Contact request query helpers
//!
//! This module provides helper functions for querying contact requests from the platform.
//!
//! The fetch is **incremental and fully paginated**: an optional `after_created_at`
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

/// Page size for the paginated contact-request fetch. The fetch drains pages
/// (retrieve-all, up to the per-sweep budget); this only bounds how many
/// documents move per round trip. 100 is the platform document-query maximum.
const CONTACT_REQUEST_PAGE_SIZE: u32 = 100;

/// Per-sweep page budget. `contactRequest` documents are public and freely
/// indexable by `toUserId`, so a hostile sender can flood a target with cheap
/// throwaway requests; without a cap, every cold-start / restore sweep would
/// fetch and hold the entire spam set in memory at once. The fetch is
/// `$createdAt`-ascending and the caller's high-water cursor resumes from the
/// max `$createdAt` fetched, so capping pages spreads a large backlog across
/// sweeps oldest-first — nothing is buried or skipped, only deferred. 50 × 100
/// = 5_000 documents per sweep, far above any legitimate pending-request count
/// (and a legit user above it just takes an extra sweep to fully ingest).
///
/// Forward progress assumes no single `$createdAt` value holds ≥ this budget of
/// matching documents. `$createdAt` is block-granular (every doc in a block
/// shares the block time), so a same-`$createdAt` cluster is bounded by one
/// block's transaction capacity — far below 5_000 fee-paid, signed
/// `contactRequest`s. If that ever ceased to hold, the timestamp cursor could
/// not advance past such a cluster (it would re-read the same oldest 5_000 each
/// sweep); the fix would be a persisted `StartAfter` document-id continuation
/// cursor rather than the `$createdAt` high-water.
///
/// The wallet caller widens that single-cluster case into a time *window*: it
/// rewinds each sweep's lower `$createdAt` bound by a 10-minute overlap
/// (`SYNC_OVERLAP_MS`) for clock-skew / page-boundary safety. So an attacker
/// who concentrates ≥ this budget of `contactRequest`s within any 10-minute
/// span targeting one recipient (≥5_000 funded `(ownerId, toUserId)` pairs —
/// costly but reachable at scale) keeps the high-water pinned at the window's
/// max `$createdAt`, and the next sweep's rewind lands back inside the same
/// window — the same non-advancing cursor, with a wider trigger. The memory
/// bound still holds (oldest-first, budget-capped); only forward progress past
/// a fully saturated window stalls, and the same `StartAfter` document-id
/// continuation cursor is the recovery.
const MAX_CONTACT_REQUEST_PAGES_PER_SWEEP: u32 = 50;

impl Sdk {
    /// Drain `contactRequest` documents matching `filter_field ==
    /// identity_id` (and, if `after_created_at` is set, `$createdAt >
    /// after_created_at`), paginating with a `StartAfter` document-id cursor
    /// until a short/empty page proves exhaustion **or** the per-sweep page
    /// budget ([`MAX_CONTACT_REQUEST_PAGES_PER_SWEEP`]) is hit.
    ///
    /// `Ok` ⇒ the fetch completed without error and the caller may advance its
    /// high-water mark to the max `$createdAt` fetched; a page error
    /// short-circuits as `Err` so the caller does not advance. The result may
    /// be a budgeted PARTIAL (oldest-first) under a `toUserId` flood — the
    /// high-water cursor resumes the remainder on the next sweep.
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
        let mut pages_fetched: u32 = 0;

        loop {
            let query = DocumentQuery {
                select: drive::query::SelectProjection::documents(),
                data_contract: dashpay_contract.clone(),
                document_type_name: "contactRequest".to_string(),
                where_clauses: where_clauses.clone(),
                time_range_clauses: vec![],
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
                offset: None,
                start: start.clone(),
            };

            let page = Document::fetch_many(self, query).await?;
            let page_len = page.len();
            // The last document id in query order seeds the next page's
            // cursor (distinct from the `$createdAt` high-water the caller
            // tracks — this id cursor is ephemeral, per-loop). Relies on
            // `Documents` being insertion-ordered (`IndexMap`) so `keys().last()`
            // is the `$createdAt`-ascending last doc; a `BTreeMap` here would
            // silently reorder by doc id and break pagination.
            let last_id = page.keys().last().copied();
            for (id, doc) in page {
                all.insert(id, doc);
            }
            pages_fetched += 1;

            // Stop on a short page (exhaustion) or the per-sweep page budget. A
            // budget stop on a still-full page is logged: `all` holds the oldest
            // requests (`$createdAt ASC`), the caller advances its high-water
            // cursor to the max fetched, and the next sweep resumes from here —
            // the backlog drains oldest-first across sweeps, never buried.
            if !should_fetch_another_contact_request_page(page_len, pages_fetched) {
                if page_len >= CONTACT_REQUEST_PAGE_SIZE as usize {
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

/// Whether the contact-request pagination loop should fetch another page:
/// only when the last page was full (more may remain) AND the per-sweep page
/// budget has not been reached. A short page (exhaustion) stops first and
/// takes priority over the budget.
fn should_fetch_another_contact_request_page(last_page_len: usize, pages_fetched: u32) -> bool {
    last_page_len >= CONTACT_REQUEST_PAGE_SIZE as usize
        && pages_fetched < MAX_CONTACT_REQUEST_PAGES_PER_SWEEP
}

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
