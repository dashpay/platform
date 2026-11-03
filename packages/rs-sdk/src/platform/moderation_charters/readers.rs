//! Proved document queries on the moderation charters contract.

use super::ModerationTeam;
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::{DataContract, Document, Fetch, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::document::DocumentV0Getters;
use dpp::moderation_charter::{
    property_names, ElectedCharter, ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
    ELECTED_CHARTER_DOCUMENT_TYPE_NAME, JOIN_REQUEST_DOCUMENT_TYPE_NAME,
    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME, RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
    SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::{Identifier, Value};
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;

/// The most documents one query returns, the platform's cap.
const MAX_PAGE_SIZE: u32 = 100;

/// How many full pages a reader that must see every document (the team's additions and
/// removals, a charter's resignation requests) fetches before it gives up rather than answer
/// from part of the set. Nothing caps additions per charter until seating lands (it will
/// allow at most 15), so the budget only bounds a loop over a set a leader could grow at will.
const MAX_PAGES_PER_READ: u32 = 50;

/// One page of a listing: at most `limit` documents (the platform's cap of 100 when `None` or
/// 0), after the document `start_after` in the index's order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharterDocumentsPage {
    /// How many documents at most; `None` and 0 are 100.
    pub limit: Option<u32>,
    /// The id of the last document of the previous page; `None` for the first page.
    pub start_after: Option<Identifier>,
}

impl CharterDocumentsPage {
    /// The page after the one `page` holds, `None` when `page` was the last: it held fewer
    /// documents than the limit.
    pub fn after(&self, page: &Documents) -> Option<Self> {
        let limit = self.page_size();
        if page.len() < limit as usize {
            return None;
        }
        page.keys().last().map(|last| Self {
            limit: self.limit,
            start_after: Some(*last),
        })
    }

    /// The number of documents a page holds when it is full.
    fn page_size(&self) -> u32 {
        match self.limit {
            None | Some(0) => MAX_PAGE_SIZE,
            Some(limit) => limit,
        }
    }
}

/// A contract's seated charter: its `electedCharter` document and the properties read out of
/// it. Its owner is the leader.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatedCharter {
    /// The `electedCharter` document.
    pub document: Document,
    /// Its properties: the target, the proposal and the elected members.
    pub charter: ElectedCharter,
}

impl SeatedCharter {
    /// Reads a seated charter out of an `electedCharter` document.
    pub fn from_document(document: Document) -> Result<Self, Error> {
        let charter = elected_charter_of(&document)?;
        Ok(Self { document, charter })
    }

    /// The id of the `electedCharter` document.
    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    /// The leader: the owner of the charter, and of its proposal.
    pub fn leader_id(&self) -> Identifier {
        self.document.owner_id()
    }
}

/// Reads the properties of an `electedCharter` document.
pub(super) fn elected_charter_of(document: &Document) -> Result<ElectedCharter, Error> {
    let result = ElectedCharter::from_document_properties(document.properties());
    if let Some(error) = result.errors.first() {
        return Err(Error::Generic(format!(
            "electedCharter {} is malformed: {error}",
            document.id()
        )));
    }
    result.into_data().map_err(Error::Protocol)
}

/// A query for the documents of `document_type_name` whose `field` is `value`, in the order of
/// `order_by` (the index property after `field`), one page of them.
pub(super) fn index_query(
    contract: Arc<DataContract>,
    document_type_name: &str,
    field: &str,
    value: Identifier,
    order_by: Option<&str>,
    page: CharterDocumentsPage,
) -> Result<DocumentQuery, Error> {
    let mut query = DocumentQuery::new(contract, document_type_name)?
        .with_where(WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(value.to_buffer()),
        })
        .with_limit(page.page_size());
    // The order clause pins the index: a bare equality on the first property of a
    // two-property index is proven absent instead of served.
    if let Some(order_by) = order_by {
        query = query.with_order_by(OrderClause {
            field: order_by.to_string(),
            ascending: true,
        });
    }
    query.start = page
        .start_after
        .map(|id| Start::StartAfter(id.to_buffer().to_vec()));
    Ok(query)
}

/// The query for the seated charter of `target_contract_id`: its `electedCharter` through the
/// contested unique index `byTargetContract`.
pub(super) fn seated_charter_query(
    contract: Arc<DataContract>,
    target_contract_id: Identifier,
) -> Result<DocumentQuery, Error> {
    index_query(
        contract,
        ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
        property_names::TARGET_CONTRACT_ID,
        target_contract_id,
        None,
        CharterDocumentsPage {
            limit: Some(1),
            start_after: None,
        },
    )
}

/// The queries for a charter's additions and removals, `byElectedCharterMember` on each type.
pub(super) fn team_change_query(
    contract: Arc<DataContract>,
    document_type_name: &str,
    elected_charter_id: Identifier,
    page: CharterDocumentsPage,
) -> Result<DocumentQuery, Error> {
    index_query(
        contract,
        document_type_name,
        property_names::ELECTED_CHARTER_ID,
        elected_charter_id,
        Some(property_names::MEMBER_ID),
        page,
    )
}

/// The query for a charter's resignation requests, `byElectedCharterOwner`.
pub(super) fn resignation_requests_query(
    contract: Arc<DataContract>,
    elected_charter_id: Identifier,
    page: CharterDocumentsPage,
) -> Result<DocumentQuery, Error> {
    index_query(
        contract,
        RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
        property_names::ELECTED_CHARTER_ID,
        elected_charter_id,
        Some("$ownerId"),
        page,
    )
}

/// The query for the proposals for a contract, `byTargetContract` in filing order.
pub(super) fn submitted_charters_query(
    contract: Arc<DataContract>,
    target_contract_id: Identifier,
    page: CharterDocumentsPage,
) -> Result<DocumentQuery, Error> {
    index_query(
        contract,
        SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
        property_names::TARGET_CONTRACT_ID,
        target_contract_id,
        Some("$createdAt"),
        page,
    )
}

/// The query for the join requests for a proposal, `bySubmittedCharter` in owner order.
pub(super) fn join_requests_query(
    contract: Arc<DataContract>,
    submitted_charter_id: Identifier,
    page: CharterDocumentsPage,
) -> Result<DocumentQuery, Error> {
    index_query(
        contract,
        JOIN_REQUEST_DOCUMENT_TYPE_NAME,
        property_names::SUBMITTED_CHARTER_ID,
        submitted_charter_id,
        Some("$ownerId"),
        page,
    )
}

/// The resignation requests among `requests` whose writer is still on `team`: the ones the
/// leader has not acted on, by deleting the writer's addition or removing an elected writer.
pub(super) fn pending_resignation_requests(
    requests: Vec<Document>,
    team: &ModerationTeam,
) -> Vec<Document> {
    requests
        .into_iter()
        .filter(|request| team.contains(&request.owner_id()))
        .collect()
}

/// The `memberId` of each of `documents`.
pub(super) fn member_ids(documents: &[Document]) -> Result<BTreeSet<Identifier>, Error> {
    documents
        .iter()
        .map(|document| {
            document
                .properties()
                .get(property_names::MEMBER_ID)
                .ok_or_else(|| {
                    Error::Generic(format!("document {} has no memberId", document.id()))
                })?
                .to_identifier()
                .map_err(|e| Error::Generic(format!("document {}: {e}", document.id())))
        })
        .collect()
}

impl Sdk {
    /// The seated charter of `target_contract_id`: the `electedCharter` whose
    /// `targetContractId` is it, `None` when the contract has none (no contest awarded yet, or
    /// no elected moderation at all).
    pub async fn fetch_seated_charter(
        &self,
        target_contract_id: Identifier,
    ) -> Result<Option<SeatedCharter>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        self.fetch_seated_charter_of(contract, target_contract_id)
            .await
    }

    async fn fetch_seated_charter_of(
        &self,
        contract: Arc<DataContract>,
        target_contract_id: Identifier,
    ) -> Result<Option<SeatedCharter>, Error> {
        let documents =
            Document::fetch_many(self, seated_charter_query(contract, target_contract_id)?).await?;
        documents
            .into_values()
            .flatten()
            .next()
            .map(SeatedCharter::from_document)
            .transpose()
    }

    /// The `submittedCharter` (a proposal) with id `submitted_charter_id`, `None` when there is
    /// none.
    pub async fn fetch_submitted_charter(
        &self,
        submitted_charter_id: Identifier,
    ) -> Result<Option<Document>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        self.fetch_submitted_charter_of(contract, submitted_charter_id)
            .await
    }

    /// [`Sdk::fetch_submitted_charter`] with the charters contract already resolved.
    pub(super) async fn fetch_submitted_charter_of(
        &self,
        contract: Arc<DataContract>,
        submitted_charter_id: Identifier,
    ) -> Result<Option<Document>, Error> {
        let query = DocumentQuery::new(contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME)?
            .with_document_id(&submitted_charter_id);
        Document::fetch(self, query).await
    }

    /// The `electedCharter` with id `elected_charter_id`, `None` when there is none. A stored
    /// elected charter is always a seated one.
    pub async fn fetch_elected_charter(
        &self,
        elected_charter_id: Identifier,
    ) -> Result<Option<SeatedCharter>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        self.fetch_elected_charter_of(contract, elected_charter_id)
            .await
    }

    /// [`Sdk::fetch_elected_charter`] with the charters contract already resolved.
    pub(super) async fn fetch_elected_charter_of(
        &self,
        contract: Arc<DataContract>,
        elected_charter_id: Identifier,
    ) -> Result<Option<SeatedCharter>, Error> {
        let query = DocumentQuery::new(contract, ELECTED_CHARTER_DOCUMENT_TYPE_NAME)?
            .with_document_id(&elected_charter_id);
        Document::fetch(self, query)
            .await?
            .map(SeatedCharter::from_document)
            .transpose()
    }

    /// The team that moderates `target_contract_id`: the seated charter's leader plus its
    /// elected members and the members the leader added, less those the leader removed.
    /// `None` when the contract has no seated charter.
    pub async fn fetch_moderation_team(
        &self,
        target_contract_id: Identifier,
    ) -> Result<Option<ModerationTeam>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        let Some(seated) = self
            .fetch_seated_charter_of(contract.clone(), target_contract_id)
            .await?
        else {
            return Ok(None);
        };
        let (added, removed) = futures::try_join!(
            self.fetch_every_page(|page| team_change_query(
                contract.clone(),
                ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                seated.id(),
                page,
            )),
            self.fetch_every_page(|page| team_change_query(
                contract.clone(),
                REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                seated.id(),
                page,
            )),
        )?;
        ModerationTeam::from_documents(&seated.document, &added, &removed).map(Some)
    }

    /// One page of the proposals (`submittedCharter` documents) for `target_contract_id`, in
    /// filing order. [`CharterDocumentsPage::after`] gives the next page.
    pub async fn fetch_submitted_charters(
        &self,
        target_contract_id: Identifier,
        page: CharterDocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        Document::fetch_many(
            self,
            submitted_charters_query(contract, target_contract_id, page)?,
        )
        .await
    }

    /// One page of the join requests for the proposal `submitted_charter_id`, in the order of
    /// their owners' ids. [`CharterDocumentsPage::after`] gives the next page.
    pub async fn fetch_join_requests(
        &self,
        submitted_charter_id: Identifier,
        page: CharterDocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        Document::fetch_many(
            self,
            join_requests_query(contract, submitted_charter_id, page)?,
        )
        .await
    }

    /// The resignation requests for the charter `elected_charter_id` the leader has not acted
    /// on: those whose writer is still on the team (the leader takes an added member off by
    /// deleting its `addedModerator`, and an elected one with a `removedModerator`). A
    /// withdrawn request is deleted, so it is not among them either. Empty when there is no
    /// such charter.
    pub async fn fetch_pending_resignation_requests(
        &self,
        elected_charter_id: Identifier,
    ) -> Result<Vec<Document>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        let Some(seated) = self
            .fetch_elected_charter_of(contract.clone(), elected_charter_id)
            .await?
        else {
            return Ok(vec![]);
        };
        let (requests, added, removed) = futures::try_join!(
            self.fetch_every_page(|page| resignation_requests_query(
                contract.clone(),
                elected_charter_id,
                page,
            )),
            self.fetch_every_page(|page| team_change_query(
                contract.clone(),
                ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                elected_charter_id,
                page,
            )),
            self.fetch_every_page(|page| team_change_query(
                contract.clone(),
                REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                elected_charter_id,
                page,
            )),
        )?;
        let team = ModerationTeam::from_documents(&seated.document, &added, &removed)?;
        Ok(pending_resignation_requests(requests, &team))
    }

    /// Every document the query `query_for_page` builds matches, page after page, or an error
    /// when there are more than [`MAX_PAGES_PER_READ`] pages of them: a reader that answers for
    /// the whole set must not answer from part of it.
    async fn fetch_every_page(
        &self,
        query_for_page: impl Fn(CharterDocumentsPage) -> Result<DocumentQuery, Error>,
    ) -> Result<Vec<Document>, Error> {
        collect_every_page(|page| {
            let query = query_for_page(page);
            async move { Document::fetch_many(self, query?).await }
        })
        .await
    }
}

/// Every document `fetch_page` returns, page after page, up to [`MAX_PAGES_PER_READ`] full
/// pages. With the budget spent, one more page is read: empty, the set held exactly the budget
/// and is complete; not empty, the set is larger and the read is refused.
pub(super) async fn collect_every_page<F, Fut>(mut fetch_page: F) -> Result<Vec<Document>, Error>
where
    F: FnMut(CharterDocumentsPage) -> Fut,
    Fut: Future<Output = Result<Documents, Error>>,
{
    let mut documents = Vec::new();
    let mut page = CharterDocumentsPage::default();
    for _ in 0..MAX_PAGES_PER_READ {
        let fetched = fetch_page(page).await?;
        let next = page.after(&fetched);
        documents.extend(fetched.into_values().flatten());
        match next {
            Some(next) => page = next,
            None => return Ok(documents),
        }
    }
    if fetch_page(page).await?.is_empty() {
        return Ok(documents);
    }
    Err(Error::Generic(format!(
        "more than {} moderation charter documents match; refusing to answer from part of them",
        MAX_PAGES_PER_READ * MAX_PAGE_SIZE
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::document::DocumentV0;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use drive::query::DriveDocumentQuery;
    use std::collections::BTreeMap;
    use std::future::{ready, Ready};

    fn charters_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(
                SystemDataContract::ModerationCharters,
                PlatformVersion::latest(),
            )
            .expect("the moderation charters contract loads"),
        )
    }

    /// The index `query` is served through, and that a proof of it can be verified: the path
    /// query the verifier rebuilds constructs.
    fn served_through(query: &DocumentQuery) -> String {
        let platform_version = PlatformVersion::latest();
        let drive_query = DriveDocumentQuery::try_from(query).expect("converts");
        drive_query
            .construct_path_query(None, platform_version)
            .expect("a provable path query");
        drive_query
            .find_best_index(platform_version)
            .expect("an index serves it")
            .name
            .clone()
    }

    #[test]
    fn should_serve_every_reader_through_the_index_the_schema_declares_for_it() {
        let contract = charters_contract();
        let id = Identifier::from([7; 32]);
        let page = CharterDocumentsPage::default();
        let cases = [
            (
                seated_charter_query(contract.clone(), id).expect("builds"),
                "byTargetContract",
            ),
            (
                submitted_charters_query(contract.clone(), id, page).expect("builds"),
                "byTargetContract",
            ),
            (
                join_requests_query(contract.clone(), id, page).expect("builds"),
                "bySubmittedCharter",
            ),
            (
                team_change_query(
                    contract.clone(),
                    ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                    id,
                    page,
                )
                .expect("builds"),
                "byElectedCharterMember",
            ),
            (
                team_change_query(
                    contract.clone(),
                    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                    id,
                    page,
                )
                .expect("builds"),
                "byElectedCharterMember",
            ),
            (
                resignation_requests_query(contract.clone(), id, page).expect("builds"),
                "byElectedCharterOwner",
            ),
        ];
        for (query, index) in cases {
            assert_eq!(
                served_through(&query),
                index,
                "{} query",
                query.document_type_name
            );
        }
        // The contested index is the elected charter's own
        assert!(contract
            .document_type_for_name(ELECTED_CHARTER_DOCUMENT_TYPE_NAME)
            .expect("exists")
            .indexes()
            .get("byTargetContract")
            .expect("declared")
            .contested_index
            .is_some());
    }

    #[test]
    fn should_page_after_the_last_document_of_a_full_page_only() {
        let page = CharterDocumentsPage {
            limit: Some(2),
            start_after: None,
        };
        let full: Documents = [Identifier::from([1; 32]), Identifier::from([2; 32])]
            .into_iter()
            .map(|id| (id, None))
            .collect();
        assert_eq!(
            page.after(&full),
            Some(CharterDocumentsPage {
                limit: Some(2),
                start_after: Some(Identifier::from([2; 32])),
            })
        );
        let short: Documents = [(Identifier::from([1; 32]), None)].into_iter().collect();
        assert_eq!(page.after(&short), None);
        // A limit of 0 is the platform's default of 100, so two documents are a short page
        let default_size = CharterDocumentsPage {
            limit: Some(0),
            start_after: None,
        };
        assert_eq!(default_size.after(&full), None);
    }

    fn document(id: u32, owner: u8) -> Document {
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(&id.to_be_bytes());
        DocumentV0 {
            id: Identifier::from(bytes),
            owner_id: Identifier::from([owner; 32]),
            ..Default::default()
        }
        .into()
    }

    /// A store of `total` documents served a page at a time, as the platform pages them.
    fn serve_pages(
        total: u32,
    ) -> impl FnMut(CharterDocumentsPage) -> Ready<Result<Documents, Error>> {
        move |page| {
            let first = match page.start_after {
                None => 0,
                Some(last) => {
                    let bytes = last.to_buffer();
                    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) + 1
                }
            };
            let end = total.min(first + page.page_size());
            ready(Ok((first..end)
                .map(|id| {
                    let document = document(id, 1);
                    (document.id(), Some(document))
                })
                .collect()))
        }
    }

    #[tokio::test]
    async fn should_read_a_set_of_exactly_the_page_budget_in_full() {
        let budget = MAX_PAGES_PER_READ * MAX_PAGE_SIZE;
        let documents = collect_every_page(serve_pages(budget))
            .await
            .expect("a set of exactly the budget is complete");
        assert_eq!(documents.len(), budget as usize);

        let short = collect_every_page(serve_pages(250)).await.expect("reads");
        assert_eq!(short.len(), 250);
    }

    #[tokio::test]
    async fn should_refuse_a_set_larger_than_the_page_budget() {
        let budget = MAX_PAGES_PER_READ * MAX_PAGE_SIZE;
        assert!(collect_every_page(serve_pages(budget + 1)).await.is_err());
    }

    #[test]
    fn should_keep_only_the_resignation_requests_whose_writer_is_still_on_the_team() {
        let requests = vec![document(1, 0xA1), document(2, 0xA2), document(3, 0xA3)];
        // 0xA2 was taken off: an elected member removed, or an added one whose addition
        // was deleted, is no longer among the members either way
        let team = ModerationTeam {
            elected_charter_id: Identifier::from([0xE1; 32]),
            submitted_charter_id: Identifier::from([0xE2; 32]),
            leader_id: Identifier::from([0xE3; 32]),
            members: BTreeSet::from([Identifier::from([0xA1; 32]), Identifier::from([0xA3; 32])]),
        };

        let pending = pending_resignation_requests(requests, &team);

        assert_eq!(
            pending
                .iter()
                .map(|request| request.owner_id())
                .collect::<Vec<_>>(),
            vec![Identifier::from([0xA1; 32]), Identifier::from([0xA3; 32])]
        );
        assert!(pending_resignation_requests(vec![], &team).is_empty());
    }

    #[test]
    fn should_read_the_member_of_each_team_change() {
        let change = |member: u8| -> Document {
            DocumentV0 {
                id: Identifier::from([member; 32]),
                properties: BTreeMap::from([(
                    property_names::MEMBER_ID.to_string(),
                    Value::Identifier([member; 32]),
                )]),
                ..Default::default()
            }
            .into()
        };
        assert_eq!(
            member_ids(&[change(5), change(6)]).expect("reads"),
            BTreeSet::from([Identifier::from([5; 32]), Identifier::from([6; 32])])
        );
        assert!(member_ids(&[document(1, 1)]).is_err());
    }

    /// Serves `documents` a page at a time in their given order, the order of the index a query
    /// walks, continuing after the document a page names.
    fn serve_in_order(
        documents: Vec<Document>,
    ) -> impl FnMut(CharterDocumentsPage) -> Ready<Result<Documents, Error>> {
        move |page| {
            let first = match page.start_after {
                None => 0,
                Some(last) => {
                    documents
                        .iter()
                        .position(|document| document.id() == last)
                        .expect("the cursor names a served document")
                        + 1
                }
            };
            let end = documents.len().min(first + page.page_size() as usize);
            ready(Ok(documents[first..end]
                .iter()
                .map(|document| (document.id(), Some(document.clone())))
                .collect()))
        }
    }

    #[tokio::test]
    async fn should_continue_after_the_last_document_of_each_page_in_query_order() {
        // Ids run against the index order, so the last document of a page is not its largest id
        let documents: Vec<Document> = (0..102u32).rev().map(|id| document(id, 1)).collect();
        let mut serve = serve_in_order(documents.clone());
        let mut cursors = Vec::new();
        let collected = collect_every_page(|page: CharterDocumentsPage| {
            cursors.push(page.start_after);
            serve(page)
        })
        .await
        .expect("reads");

        assert_eq!(cursors, vec![None, Some(documents[99].id())]);
        assert_eq!(
            collected
                .iter()
                .map(|document| document.id())
                .collect::<Vec<_>>(),
            documents
                .iter()
                .map(|document| document.id())
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn should_build_the_team_from_changes_on_every_page() {
        let charter_id = Identifier::from([0xC1; 32]);
        let leader = Identifier::from([0x01; 32]);
        let member = |n: u32| {
            let mut bytes = [0xEE; 32];
            bytes[..4].copy_from_slice(&n.to_be_bytes());
            Identifier::from(bytes)
        };
        let change = |n: u32, id_byte: u8| -> Document {
            let mut id = [id_byte; 32];
            id[..4].copy_from_slice(&n.to_be_bytes());
            DocumentV0 {
                id: Identifier::from(id),
                owner_id: leader,
                properties: BTreeMap::from([
                    (
                        property_names::ELECTED_CHARTER_ID.to_string(),
                        Value::Identifier(charter_id.to_buffer()),
                    ),
                    (
                        property_names::MEMBER_ID.to_string(),
                        Value::Identifier(member(n).to_buffer()),
                    ),
                ]),
                ..Default::default()
            }
            .into()
        };
        let charter: Document = DocumentV0 {
            id: charter_id,
            owner_id: leader,
            properties: ElectedCharter {
                target_contract_id: Identifier::from([0xAA; 32]),
                submitted_charter_id: Identifier::from([0xBB; 32]),
                members: vec![member(1000), member(1001)],
            }
            .to_document_properties(),
            ..Default::default()
        }
        .into();

        // 105 additions over two pages; 101 removals over two pages, the last one on the
        // second page removing an elected member
        let added = collect_every_page(serve_in_order((0..105).map(|n| change(n, 0xA0)).collect()))
            .await
            .expect("reads the additions");
        let removed = collect_every_page(serve_in_order(
            (0..100)
                .map(|n| change(n, 0xB0))
                .chain([change(1001, 0xB1)])
                .collect(),
        ))
        .await
        .expect("reads the removals");
        assert_eq!((added.len(), removed.len()), (105, 101));

        let team = ModerationTeam::from_documents(&charter, &added, &removed).expect("reads");
        let mut expected: BTreeSet<Identifier> = (100..105).map(member).collect();
        expected.insert(member(1000));
        assert_eq!(team.members, expected);
    }
}
