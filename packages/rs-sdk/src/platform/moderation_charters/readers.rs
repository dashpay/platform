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

/// The resignation requests among `requests` whose writer is not among `removed`, the
/// `memberId`s of the charter's removals: the ones the leader has not acted on.
pub(super) fn pending_resignation_requests(
    requests: Vec<Document>,
    removed: &BTreeSet<Identifier>,
) -> Vec<Document> {
    requests
        .into_iter()
        .filter(|request| !removed.contains(&request.owner_id()))
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
    /// on: those whose writer the charter has no `removedModerator` for. A withdrawn request is
    /// deleted, so it is not among them either.
    pub async fn fetch_pending_resignation_requests(
        &self,
        elected_charter_id: Identifier,
    ) -> Result<Vec<Document>, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        let (requests, removed) = futures::try_join!(
            self.fetch_every_page(|page| resignation_requests_query(
                contract.clone(),
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
        Ok(pending_resignation_requests(
            requests,
            &member_ids(&removed)?,
        ))
    }

    /// Every document the query `query_for_page` builds matches, page after page, or an error
    /// when there are more than [`MAX_PAGES_PER_READ`] pages of them: a reader that answers for
    /// the whole set must not answer from part of it.
    async fn fetch_every_page(
        &self,
        query_for_page: impl Fn(CharterDocumentsPage) -> Result<DocumentQuery, Error>,
    ) -> Result<Vec<Document>, Error> {
        let mut documents = Vec::new();
        let mut page = CharterDocumentsPage::default();
        for _ in 0..MAX_PAGES_PER_READ {
            let fetched = Document::fetch_many(self, query_for_page(page)?).await?;
            let next = page.after(&fetched);
            documents.extend(fetched.into_values().flatten());
            match next {
                Some(next) => page = next,
                None => return Ok(documents),
            }
        }
        Err(Error::Generic(format!(
            "more than {} moderation charter documents match; refusing to answer from part of \
             them",
            MAX_PAGES_PER_READ * MAX_PAGE_SIZE
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use drive::query::DriveDocumentQuery;

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
}
