use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::{Document, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use drive::query::{OrderClause, SelectProjection, WhereClause, WhereOperator};
use std::sync::Arc;

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
            integer_range_clauses: vec![],
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
    /// the end was not provably reached (the caller must then treat the
    /// result as a lower bound, not the owned set). Documents that do not
    /// parse as DPNS domains are skipped, as in
    /// [`Self::get_dpns_usernames_by_identity`].
    ///
    /// # When `true` can be trusted
    ///
    /// Continuing a page relies on `startAfter` inside the `records.identity`
    /// equality bucket, which only the query lowering introduced with
    /// protocol version 14 (`non_primary_key_path_query: 1`) serves. The
    /// earlier lowering drops the rest of the bucket after any cursor, so a
    /// second page would come back empty and read as the end. On such a
    /// platform version (the one the SDK runs at once the first page has
    /// answered) the loop reads only the first page: `true` then means the
    /// identity has fewer than `page_limit` names, and a full first page
    /// returns `false`. Otherwise `false` means `max_pages` full pages were
    /// read without reaching the end.
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
            let query = identity_domains_page_query(
                dpns_contract.clone(),
                identity_id,
                page_limit,
                start_after,
            );
            let documents = Document::fetch_many(self, query).await?;
            // Termination and the cursor follow the RAW page (every returned
            // entry, parseable or not), never the converted usernames: a
            // skipped document still counts toward a full page and still
            // moves the cursor past it.
            let page_len = documents.len();
            let last_id = documents.keys().last().copied();
            // Read after the fetch: the response has already moved an
            // auto-detecting SDK to the network's protocol version.
            let cursor_continues = start_after_continues_an_equality_bucket(self.version());
            usernames.extend(
                documents
                    .into_values()
                    .flatten()
                    .filter_map(Self::document_to_dpns_username),
            );
            match next_dpns_page(page_len, page_limit, last_id, cursor_continues) {
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
            integer_range_clauses: vec![],
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

/// One page of `domain` documents whose `records.identity` is `identity_id`,
/// resuming after `start_after` when set.
fn identity_domains_page_query(
    dpns_contract: Arc<DataContract>,
    identity_id: Identifier,
    page_limit: u32,
    start_after: Option<Identifier>,
) -> DocumentQuery {
    DocumentQuery {
        select: SelectProjection::documents(),
        data_contract: dpns_contract,
        document_type_name: "domain".to_string(),
        where_clauses: vec![WhereClause {
            field: "records.identity".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(identity_id.to_buffer()),
        }],
        time_range_clauses: vec![],
        integer_range_clauses: vec![],
        sub_queries: vec![],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![],
        limit: page_limit,
        offset: None,
        start: start_after.map(|id| Start::StartAfter(id.to_vec())),
    }
}

/// What to do after one page of a paged DPNS username query.
#[derive(Debug, PartialEq, Eq)]
enum DpnsPageStep {
    /// A short page: the whole set has been read.
    Complete,
    /// A full page: continue after this document id.
    Continue(Identifier),
    /// A full page that cannot be continued (no cursor, or a platform
    /// version whose lowering drops the rest of the bucket after one):
    /// completeness cannot be established.
    Unknown,
}

fn next_dpns_page(
    page_len: usize,
    page_limit: u32,
    last_id: Option<Identifier>,
    cursor_continues: bool,
) -> DpnsPageStep {
    if page_len < page_limit as usize {
        DpnsPageStep::Complete
    } else if let (Some(id), true) = (last_id, cursor_continues) {
        DpnsPageStep::Continue(id)
    } else {
        DpnsPageStep::Unknown
    }
}

/// Whether `startAfter` a document inside an equality bucket (no `orderBy`)
/// returns the rest of that bucket. The v0 non-primary-key lowering
/// (protocol versions <= 13) hands the exclusive cursor to the equality
/// key itself and so drops the whole bucket; the v1 lowering continues on
/// the document-id level. Pinned in Drive by the
/// `dpns_identity_records_paging` tests in `rs-drive/tests/query_tests.rs`.
fn start_after_continues_an_equality_bucket(platform_version: &PlatformVersion) -> bool {
    platform_version
        .drive
        .methods
        .document
        .query
        .non_primary_key_path_query
        != 0
}

#[cfg(test)]
mod paging_tests {
    use super::*;

    #[test]
    fn a_short_page_completes_the_set() {
        for cursor_continues in [true, false] {
            assert_eq!(
                next_dpns_page(0, 100, None, cursor_continues),
                DpnsPageStep::Complete
            );
            assert_eq!(
                next_dpns_page(99, 100, Some(Identifier::from([1u8; 32])), cursor_continues),
                DpnsPageStep::Complete
            );
        }
    }

    #[test]
    fn a_full_page_continues_after_its_last_document() {
        let last = Identifier::from([7u8; 32]);
        assert_eq!(
            next_dpns_page(100, 100, Some(last), true),
            DpnsPageStep::Continue(last)
        );
    }

    #[test]
    fn a_full_page_without_a_cursor_is_not_complete() {
        assert_eq!(next_dpns_page(100, 100, None, true), DpnsPageStep::Unknown);
    }

    #[test]
    fn a_full_page_on_a_lowering_that_drops_the_bucket_is_not_complete() {
        assert_eq!(
            next_dpns_page(100, 100, Some(Identifier::from([7u8; 32])), false),
            DpnsPageStep::Unknown
        );
    }

    #[test]
    fn only_the_v1_lowering_continues_an_equality_bucket() {
        for protocol_version in 1..=13 {
            let version = PlatformVersion::get(protocol_version).expect("released version");
            assert!(
                !start_after_continues_an_equality_bucket(version),
                "protocol version {protocol_version} uses the v0 lowering"
            );
        }
        assert!(start_after_continues_an_equality_bucket(
            PlatformVersion::latest()
        ));
    }
}

/// The real paging loop against a network-free mock SDK. Every expectation
/// is keyed by the exact encoded request, so a wrong `StartAfter` cursor, a
/// wrong limit or an extra page fails the fetch instead of passing silently.
#[cfg(all(test, feature = "mocks"))]
mod paging_loop_tests {
    use super::*;
    use crate::platform::dpns_usernames::convert_to_homograph_safe_chars;
    use crate::query_types::Documents;
    use crate::SdkBuilder;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::DataContract;
    use dpp::document::DocumentV0;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const PAGE: u32 = 2;

    fn identity() -> Identifier {
        Identifier::from([0x11; 32])
    }

    fn dpns_contract() -> Arc<DataContract> {
        let version = PlatformVersion::latest();
        let mut contract =
            dpp::tests::fixtures::get_dpns_data_contract_fixture(None, 0, version.protocol_version)
                .data_contract_owned();
        #[cfg(feature = "dpns-contract")]
        contract.set_id(dpp::system_data_contracts::SystemDataContract::DPNS.id());
        #[cfg(not(feature = "dpns-contract"))]
        contract.set_id(
            Identifier::from_string(
                "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .expect("DPNS contract id"),
        );
        Arc::new(contract)
    }

    /// A parseable `domain` document for `label`, id byte `id`.
    fn domain(id: u8, label: &str) -> (Identifier, Option<Document>) {
        let records = vec![(
            Value::Text("identity".to_string()),
            Value::Identifier(identity().to_buffer()),
        )];
        let mut properties = BTreeMap::new();
        properties.insert("label".to_string(), Value::Text(label.to_string()));
        properties.insert(
            "normalizedLabel".to_string(),
            Value::Text(convert_to_homograph_safe_chars(label)),
        );
        properties.insert(
            "normalizedParentDomainName".to_string(),
            Value::Text("dash".to_string()),
        );
        properties.insert("records".to_string(), Value::Map(records));
        let document_id = Identifier::from([id; 32]);
        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id: identity(),
            properties,
            ..Default::default()
        });
        (document_id, Some(document))
    }

    /// A document the username conversion skips (no `label`).
    fn unparseable(id: u8) -> (Identifier, Option<Document>) {
        let document_id = Identifier::from([id; 32]);
        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id: identity(),
            ..Default::default()
        });
        (document_id, Some(document))
    }

    fn page(entries: Vec<(Identifier, Option<Document>)>) -> Documents {
        entries.into_iter().collect()
    }

    /// Serves the DPNS contract (nothing else) so the loop's contract
    /// lookup never reaches the network or a dump directory.
    struct DpnsOnlyContextProvider(Arc<DataContract>);

    impl dash_context_provider::ContextProvider for DpnsOnlyContextProvider {
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
            Option<dpp::data_contract::associated_token::token_configuration::TokenConfiguration>,
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
        ) -> Result<dpp::prelude::CoreBlockHeight, crate::error::ContextProviderError> {
            Ok(1)
        }
    }

    /// Mock SDK serving the DPNS contract, with one expectation per
    /// `(start_after, page)`; requests without an expectation fail.
    async fn sdk_with_pages(pages: Vec<(Option<Identifier>, Documents)>) -> crate::Sdk {
        sdk_with_pages_at(PlatformVersion::latest(), pages).await
    }

    /// [`sdk_with_pages`] pinned to `version`.
    async fn sdk_with_pages_at(
        version: &'static PlatformVersion,
        pages: Vec<(Option<Identifier>, Documents)>,
    ) -> crate::Sdk {
        let contract = dpns_contract();
        let mut sdk = SdkBuilder::new_mock()
            .with_version(version)
            .with_context_provider(DpnsOnlyContextProvider(Arc::clone(&contract)))
            .build()
            .expect("mock sdk");
        for (start_after, documents) in pages {
            let query =
                identity_domains_page_query(Arc::clone(&contract), identity(), PAGE, start_after);
            sdk.mock()
                .expect_fetch_many::<Identifier, Document, _, Documents>(query, Some(documents))
                .await
                .expect("page expectation");
        }
        sdk
    }

    fn labels(usernames: &[DpnsUsername]) -> Vec<&str> {
        usernames.iter().map(|u| u.label.as_str()).collect()
    }

    #[tokio::test]
    async fn a_full_page_then_a_short_page_accumulates_and_completes() {
        let sdk = sdk_with_pages(vec![
            (None, page(vec![domain(1, "alice"), domain(2, "bob")])),
            (
                Some(Identifier::from([2; 32])),
                page(vec![domain(3, "carol")]),
            ),
        ])
        .await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await
            .expect("both pages answer");

        assert_eq!(labels(&usernames), ["alice", "bob", "carol"]);
        assert!(complete);
    }

    #[tokio::test]
    async fn a_full_page_then_an_empty_page_completes() {
        let sdk = sdk_with_pages(vec![
            (None, page(vec![domain(1, "alice"), domain(2, "bob")])),
            (Some(Identifier::from([2; 32])), page(Vec::new())),
        ])
        .await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await
            .expect("both pages answer");

        assert_eq!(labels(&usernames), ["alice", "bob"]);
        assert!(complete);
    }

    #[tokio::test]
    async fn exhausting_the_page_budget_is_not_complete() {
        let sdk = sdk_with_pages(vec![(
            None,
            page(vec![domain(1, "alice"), domain(2, "bob")]),
        )])
        .await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 1)
            .await
            .expect("the one page answers");

        assert_eq!(labels(&usernames), ["alice", "bob"]);
        assert!(
            !complete,
            "a full last page within the budget proves nothing"
        );
    }

    #[tokio::test]
    async fn an_error_on_a_later_page_propagates() {
        // Only the first page has an expectation; the continuation fails.
        let sdk = sdk_with_pages(vec![(
            None,
            page(vec![domain(1, "alice"), domain(2, "bob")]),
        )])
        .await;

        let result = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await;

        let error = result.expect_err("a failed page must not read as complete");
        assert!(
            !matches!(error, crate::Error::ContextProviderError(_)),
            "must fail on the unanswered second page, not on setup: {error:?}"
        );
    }

    #[tokio::test]
    async fn a_skipped_document_still_fills_the_page_and_moves_the_cursor() {
        // Page 1 is full on raw entries even though one is unparseable; the
        // continuation must start after that unparseable last document.
        let sdk = sdk_with_pages(vec![
            (None, page(vec![domain(1, "alice"), unparseable(2)])),
            (
                Some(Identifier::from([2; 32])),
                page(vec![domain(3, "carol")]),
            ),
        ])
        .await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await
            .expect("both pages answer");

        assert_eq!(labels(&usernames), ["alice", "carol"]);
        assert!(complete);
    }

    /// Protocol version 13 cannot continue inside the equality bucket, so a
    /// full first page is all the loop reads, and it is not complete. Only
    /// the first page has an expectation: a continuation would fail.
    #[tokio::test]
    async fn a_full_page_on_the_v0_lowering_stops_and_is_not_complete() {
        let v13 = PlatformVersion::get(13).expect("protocol version 13");
        let sdk = sdk_with_pages_at(
            v13,
            vec![(None, page(vec![domain(1, "alice"), domain(2, "bob")]))],
        )
        .await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await
            .expect("only the first page is requested");

        assert_eq!(labels(&usernames), ["alice", "bob"]);
        assert!(!complete, "a v0 continuation would drop the rest");
    }

    /// A short first page needs no cursor, so it is complete on any version.
    #[tokio::test]
    async fn a_short_first_page_on_the_v0_lowering_is_complete() {
        let v13 = PlatformVersion::get(13).expect("protocol version 13");
        let sdk = sdk_with_pages_at(v13, vec![(None, page(vec![domain(1, "alice")]))]).await;

        let (usernames, complete) = sdk
            .get_all_dpns_usernames_by_identity(identity(), PAGE, 10)
            .await
            .expect("the one page answers");

        assert_eq!(labels(&usernames), ["alice"]);
        assert!(complete);
    }
}
