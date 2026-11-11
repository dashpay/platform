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

/// A contact-request sweep cut back to the part a caller can resume after
/// without skipping a request; see [`Sdk::fetch_sent_contact_requests_resumable`].
#[derive(Debug, Clone)]
pub struct ResumableContactRequests {
    /// Oldest first.
    pub documents: ContactRequestDocuments,
    /// Whether these are all the requests. If not, the rest are read by a
    /// sweep with `after_created_at` set to the last document's `$createdAt`;
    /// with no documents at all, no sweep can read past the first one's.
    pub complete: bool,
}

impl Sdk {
    /// Drain `contactRequest` documents matching `filter_field ==
    /// identity_id` (and, if `after_created_at` is set, `$createdAt >
    /// after_created_at`), paginating with a `StartAfter` document-id cursor
    /// until a short/empty page proves exhaustion **or** `max_pages` (at
    /// most [`MAX_CONTACT_REQUEST_PAGES_PER_SWEEP`]) is hit.
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
        max_pages: u32,
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
                integer_range_clauses: vec![],
                sub_queries: vec![],
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
            if !should_fetch_another_contact_request_page(page_len, pages_fetched, max_pages) {
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
        self.fetch_contact_requests_paginated(
            "$ownerId",
            identity_id,
            after_created_at,
            MAX_CONTACT_REQUEST_PAGES_PER_SWEEP,
        )
        .await
    }

    /// Fetch contact requests **received** by `identity_id` (`toUserId ==`),
    /// newer than `after_created_at` if given, fully paginated.
    pub async fn fetch_received_contact_requests(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ContactRequestDocuments, Error> {
        self.fetch_contact_requests_paginated(
            "toUserId",
            identity_id,
            after_created_at,
            MAX_CONTACT_REQUEST_PAGES_PER_SWEEP,
        )
        .await
    }

    /// [`Self::fetch_sent_contact_requests`], cut back to what a caller can
    /// resume after with `after_created_at` = the last `$createdAt` read.
    pub async fn fetch_sent_contact_requests_resumable(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ResumableContactRequests, Error> {
        self.fetch_contact_requests_resumable("$ownerId", identity_id, after_created_at)
            .await
    }

    /// [`Self::fetch_received_contact_requests`], cut back to what a caller
    /// can resume after with `after_created_at` = the last `$createdAt` read.
    pub async fn fetch_received_contact_requests_resumable(
        &self,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ResumableContactRequests, Error> {
        self.fetch_contact_requests_resumable("toUserId", identity_id, after_created_at)
            .await
    }

    async fn fetch_contact_requests_resumable(
        &self,
        filter_field: &str,
        identity_id: Identifier,
        after_created_at: Option<u64>,
    ) -> Result<ResumableContactRequests, Error> {
        // Both before and after the sweep: pages served across an upgrade to
        // PV14 may include a lossy one. Before it only the first page can be
        // kept, so no other is read.
        let continues_before = cursor_continues(self);
        let max_pages = if continues_before {
            MAX_CONTACT_REQUEST_PAGES_PER_SWEEP
        } else {
            1
        };
        let documents = self
            .fetch_contact_requests_paginated(
                filter_field,
                identity_id,
                after_created_at,
                max_pages,
            )
            .await?;
        Ok(resumable_prefix(
            documents,
            continues_before && cursor_continues(self),
        ))
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
/// only when the last page was full (more may remain) AND the sweep's page
/// budget has not been reached. A short page (exhaustion) stops first and
/// takes priority over the budget.
fn should_fetch_another_contact_request_page(
    last_page_len: usize,
    pages_fetched: u32,
    max_pages: u32,
) -> bool {
    last_page_len >= CONTACT_REQUEST_PAGE_SIZE as usize && pages_fetched < max_pages
}

/// The prefix of a sweep (oldest first) a caller can resume after with
/// `after_created_at` = its last `$createdAt` without skipping a request, and
/// whether it is the whole sweep. The sweep pages on a document-id cursor that
/// is lossy at a page boundary in two cases: before protocol version 14
/// (`!cursor_continues`) a continuation skips the rest of the boundary's
/// `$createdAt`, so only the first page is whole; and a sweep cut at its page
/// budget leaves the rest of its last `$createdAt` unread. Either way the
/// requests from that `$createdAt` on are dropped, to be read again next sweep.
fn resumable_prefix(
    mut documents: ContactRequestDocuments,
    cursor_continues: bool,
) -> ResumableContactRequests {
    let page = CONTACT_REQUEST_PAGE_SIZE as usize;
    let boundary = if !cursor_continues && documents.len() >= page {
        page - 1
    } else if documents.len() >= page * MAX_CONTACT_REQUEST_PAGES_PER_SWEEP as usize {
        documents.len() - 1
    } else {
        return ResumableContactRequests {
            documents,
            complete: true,
        };
    };
    let created_at = |document: &Option<Document>| document.as_ref().and_then(|d| d.created_at());
    let cut = documents
        .get_index(boundary)
        .and_then(|(_, document)| created_at(document));
    let keep = documents
        .values()
        .position(|document| created_at(document) >= cut)
        .unwrap_or(boundary);
    documents.truncate(keep);
    ResumableContactRequests {
        documents,
        complete: false,
    }
}

/// Whether the SDK's protocol version continues a page inside a `$createdAt`
/// (PV14's query lowering).
fn cursor_continues(sdk: &Sdk) -> bool {
    sdk.version()
        .drive
        .methods
        .document
        .query
        .non_primary_key_path_query
        != 0
}

#[cfg(test)]
mod tests {
    use super::{
        resumable_prefix, should_fetch_another_contact_request_page, ContactRequestDocuments,
        CONTACT_REQUEST_PAGE_SIZE, MAX_CONTACT_REQUEST_PAGES_PER_SWEEP,
    };
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::prelude::Identifier;

    /// One document per `$createdAt`, in sweep order.
    fn sweep(created_at: impl IntoIterator<Item = u64>) -> ContactRequestDocuments {
        created_at
            .into_iter()
            .enumerate()
            .map(|(i, created_at)| {
                let mut id = [0; 32];
                id[..8].copy_from_slice(&(i as u64).to_be_bytes());
                let document = Document::V0(DocumentV0 {
                    id: Identifier::from(id),
                    created_at: Some(created_at),
                    ..Default::default()
                });
                (Identifier::from(id), Some(document))
            })
            .collect()
    }

    fn created_at(documents: ContactRequestDocuments, cursor_continues: bool) -> (Vec<u64>, bool) {
        let resumable = resumable_prefix(documents, cursor_continues);
        let created_at = resumable
            .documents
            .values()
            .map(|document| document.as_ref().and_then(|d| d.created_at()).unwrap())
            .collect();
        (created_at, resumable.complete)
    }

    /// Before PV14 the second page skips the rest of the first page's last
    /// `$createdAt` (99 here), so only the requests before it are kept; from
    /// PV14 on the sweep is whole.
    #[test]
    fn should_stop_a_sweep_before_a_lossy_page_boundary() {
        let lossy = || sweep((0..98).chain([99; 2]).chain(100..150));
        assert_eq!(created_at(lossy(), false), ((0..98).collect(), false));
        assert_eq!(
            created_at(lossy(), true),
            ((0..98).chain([99; 2]).chain(100..150).collect(), true)
        );
        // A full first page with nothing after it may still have lost its
        // last `$createdAt` before PV14.
        assert_eq!(created_at(sweep(0..100), false), ((0..99).collect(), false));
        // One short page is whole at any version.
        assert_eq!(created_at(sweep(0..99), false), ((0..99).collect(), true));
    }

    /// A sweep cut at its page budget drops its last `$createdAt`.
    #[test]
    fn should_drop_the_last_created_at_of_a_sweep_cut_at_its_budget() {
        let budget = (CONTACT_REQUEST_PAGE_SIZE * MAX_CONTACT_REQUEST_PAGES_PER_SWEEP) as u64;
        assert_eq!(
            created_at(sweep((0..budget - 2).chain([budget; 2])), true),
            ((0..budget - 2).collect(), false)
        );
    }

    /// A page of one `$createdAt` cannot be resumed after: no documents, and
    /// not complete.
    #[test]
    fn should_keep_nothing_of_a_page_sharing_one_created_at() {
        assert_eq!(created_at(sweep([5; 150]), false), (vec![], false));
    }

    #[test]
    fn pagination_continues_until_budget_then_stops() {
        let full = CONTACT_REQUEST_PAGE_SIZE as usize;
        // Full page, still under budget → keep draining.
        let budget = MAX_CONTACT_REQUEST_PAGES_PER_SWEEP;
        assert!(should_fetch_another_contact_request_page(full, 1, budget));
        assert!(should_fetch_another_contact_request_page(
            full,
            budget - 1,
            budget
        ));
        // Full page, budget reached → stop (the high-water cursor resumes the
        // remainder next sweep; a spam flood can't force an unbounded fetch).
        assert!(!should_fetch_another_contact_request_page(
            full, budget, budget
        ));
        // A one-page sweep stops after its first full page.
        assert!(!should_fetch_another_contact_request_page(full, 1, 1));
        // A short page is exhaustion — stop regardless of how few pages ran.
        assert!(!should_fetch_another_contact_request_page(
            full - 1,
            1,
            budget
        ));
        assert!(!should_fetch_another_contact_request_page(0, 1, budget));
    }

    #[cfg(feature = "mocks")]
    mod with_mock_sdk {
        use super::*;
        use crate::platform::documents::document_query::DocumentQuery;
        use crate::{Sdk, SdkBuilder};
        use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::DataContract;
        use dpp::platform_value::platform_value;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;
        use drive::query::{OrderClause, WhereClause, WhereOperator};
        use std::sync::Arc;

        /// Serves the DashPay contract (nothing else), so the sweep's contract lookup never
        /// reaches the network or a dump directory
        struct DashpayOnlyContextProvider(Arc<DataContract>);

        impl dash_context_provider::ContextProvider for DashpayOnlyContextProvider {
            fn get_data_contract(
                &self,
                id: &Identifier,
                _platform_version: &PlatformVersion,
            ) -> Result<Option<Arc<DataContract>>, crate::error::ContextProviderError> {
                Ok((*id == self.0.id()).then(|| Arc::clone(&self.0)))
            }

            fn get_token_configuration(
                &self,
                _token_id: &Identifier,
            ) -> Result<
                Option<
                    dpp::data_contract::associated_token::token_configuration::TokenConfiguration,
                >,
                crate::error::ContextProviderError,
            > {
                Ok(None)
            }

            fn get_quorum_public_key(
                &self,
                _quorum_type: u32,
                _quorum_hash: [u8; 32],
                _core_chain_locked_height: u32,
            ) -> Result<[u8; 48], crate::error::ContextProviderError> {
                Err(crate::error::ContextProviderError::Config(
                    "no quorum keys in this test".to_string(),
                ))
            }

            fn get_platform_activation_height(
                &self,
            ) -> Result<dpp::prelude::CoreBlockHeight, crate::error::ContextProviderError>
            {
                Ok(1)
            }
        }

        /// The page of requests to `recipient` from `start`, built here rather than with the
        /// code under test, so a request shaped differently matches no expectation
        fn received_page_query(
            contract: &Arc<DataContract>,
            recipient: Identifier,
            start: Option<Start>,
        ) -> DocumentQuery {
            DocumentQuery {
                select: drive::query::SelectProjection::documents(),
                data_contract: Arc::clone(contract),
                document_type_name: "contactRequest".to_string(),
                where_clauses: vec![WhereClause {
                    field: "toUserId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(recipient),
                }],
                time_range_clauses: vec![],
                integer_range_clauses: vec![],
                sub_queries: vec![],
                group_by: vec![],
                having: vec![],
                order_by_clauses: vec![OrderClause {
                    field: "$createdAt".to_string(),
                    ascending: true,
                }],
                limit: CONTACT_REQUEST_PAGE_SIZE,
                offset: None,
                start,
            }
        }

        /// Before PV14 only the first page of a sweep can be kept, so a backlog of several
        /// pages costs one request: the mock answers only the first page, and asking for the
        /// second would fail the sweep
        #[tokio::test]
        async fn should_read_one_page_of_a_resumable_sweep_before_pv14() {
            let version = PlatformVersion::get(13).expect("expected protocol version 13");
            let contract = Arc::new(
                load_system_data_contract(SystemDataContract::Dashpay, version)
                    .expect("expected the dashpay contract"),
            );
            let mut sdk: Sdk = SdkBuilder::new_mock()
                .with_version(version)
                .with_context_provider(DashpayOnlyContextProvider(Arc::clone(&contract)))
                .build()
                .expect("expected a mock sdk");
            let recipient = Identifier::new([7; 32]);
            sdk.mock()
                .expect_fetch_many::<Identifier, Document, _, ContactRequestDocuments>(
                    received_page_query(&contract, recipient, None),
                    Some(sweep(0..CONTACT_REQUEST_PAGE_SIZE as u64)),
                )
                .await
                .expect("expected to register the first page");

            let resumable = sdk
                .fetch_received_contact_requests_resumable(recipient, None)
                .await
                .expect("expected one page to answer");

            assert_eq!(
                resumable.documents.len(),
                CONTACT_REQUEST_PAGE_SIZE as usize - 1
            );
            assert!(!resumable.complete);
        }
    }
}
