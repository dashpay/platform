use super::{
    DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult, DocumentHistoryFilter,
};
use crate::drive::document::paths::{contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::common::encode::encode_u64;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::Element;

struct Fixture {
    contract: DataContract,
    document: Document,
}

impl Fixture {
    fn new() -> Self {
        let version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            false,
            version,
        )
        .expect("history-keeping profile contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile type");
        let mut document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some([8; 32].into()),
            document_type,
            version,
        )
        .expect("profile document");
        document.set_revision(Some(3));
        Self { contract, document }
    }

    fn query(
        &self,
        filter: DocumentHistoryFilter,
        limit: Option<u16>,
    ) -> DocumentHistoryDriveQuery {
        DocumentHistoryDriveQuery {
            contract_id: self.contract.id().to_buffer(),
            document_type_name: "profile".into(),
            document_id: self.document.id().to_buffer(),
            filter,
            limit,
        }
    }

    fn document_element(&self, revision: u64) -> Element {
        let mut document = self.document.clone();
        document.set_revision(Some(revision));
        Element::new_item(
            document
                .serialize(
                    self.contract
                        .document_type_for_name("profile")
                        .expect("profile type"),
                    &self.contract,
                    PlatformVersion::latest(),
                )
                .expect("serialized profile revision"),
        )
    }

    fn metadata_row(&self, branch: u8, element: Option<Element>) -> PathKeyOptionalElementTrio {
        let mut path = contract_document_type_path_vec(self.contract.id().as_bytes(), "profile");
        path.push(vec![branch]);
        (path, self.document.id().to_vec(), element)
    }

    fn entry_row(&self, revision: u64) -> PathKeyOptionalElementTrio {
        let mut path = contract_document_type_path_vec(self.contract.id().as_bytes(), "profile");
        path.push(vec![DOCUMENT_HISTORY_TREE_KEY]);
        path.push(self.document.id().to_vec());
        let mut key = encode_u64(2000);
        key.extend(encode_u64(revision));
        (path, key, Some(self.document_element(revision)))
    }

    fn rows(&self, revisions: &[u64]) -> Vec<PathKeyOptionalElementTrio> {
        let mut rows = vec![
            self.metadata_row(0, Some(self.document_element(3))),
            self.metadata_row(
                DOCUMENT_HISTORY_TREE_KEY,
                Some(Element::ProvableCountTree(Some(vec![1]), 3, None)),
            ),
        ];
        rows.extend(revisions.iter().map(|revision| self.entry_row(*revision)));
        rows
    }

    fn decode(
        &self,
        query: &DocumentHistoryDriveQuery,
        rows: Vec<PathKeyOptionalElementTrio>,
    ) -> Result<DocumentHistoryDriveQueryExecutionResult, Error> {
        query.decode_result(
            rows,
            self.contract
                .document_type_for_name("profile")
                .expect("profile type"),
            PlatformVersion::latest(),
        )
    }
}

fn assert_corrupt(
    result: Result<DocumentHistoryDriveQueryExecutionResult, Error>,
    expected: &'static str,
) {
    let error = result.expect_err("malformed history rows must be rejected");
    assert!(
        matches!(error, Error::Drive(DriveError::CorruptedDocumentPath(message)) if message == expected),
        "expected {expected:?}, got {error:?}",
    );
}

#[test]
fn should_reject_ordinal_pages_that_disagree_with_the_committed_count_or_limit() {
    let fixture = Fixture::new();
    for (filter, limit, complete, incomplete) in [
        (
            DocumentHistoryFilter::StartAtRevision(2),
            Some(10),
            vec![2, 3],
            vec![2],
        ),
        (
            DocumentHistoryFilter::StartAtRevision(2),
            Some(10),
            vec![2, 3],
            vec![2, 3, 4],
        ),
        (
            DocumentHistoryFilter::StartAtRevision(1),
            Some(2),
            vec![1, 2],
            vec![1],
        ),
        (
            DocumentHistoryFilter::StartAtRevision(1),
            Some(2),
            vec![1, 2],
            vec![1, 2, 3],
        ),
        (DocumentHistoryFilter::Revision(2), None, vec![2], vec![]),
        (
            DocumentHistoryFilter::Revision(2),
            Some(1),
            vec![2],
            vec![2, 3],
        ),
        (
            DocumentHistoryFilter::StartAtRevision(4),
            Some(10),
            vec![],
            vec![4],
        ),
        (
            DocumentHistoryFilter::Revision(65536),
            None,
            vec![],
            vec![65536],
        ),
    ] {
        let query = fixture.query(filter, limit);
        let page = fixture
            .decode(&query, fixture.rows(&complete))
            .expect("the complete ordinal page is valid");
        assert_eq!(page.entries.len(), complete.len());
        assert_corrupt(
            fixture.decode(&query, fixture.rows(&incomplete)),
            "history page length disagrees with its revision count",
        );
    }
}

#[test]
fn should_reject_time_and_cursor_pages_exceeding_their_limit_or_count() {
    let fixture = Fixture::new();
    for filter in [
        DocumentHistoryFilter::StartAtTime(0),
        DocumentHistoryFilter::StartAfter {
            time_ms: 0,
            revision: 1,
        },
    ] {
        for (limit, complete, excessive) in [
            (Some(1), vec![1], vec![1, 2]),
            (None, vec![1, 2, 3], vec![1, 2, 3, 4]),
        ] {
            let query = fixture.query(filter.clone(), limit);
            fixture
                .decode(&query, fixture.rows(&complete))
                .expect("a time page within both bounds is valid");
            assert_corrupt(
                fixture.decode(&query, fixture.rows(&excessive)),
                "history page exceeds its limit or revision count",
            );
        }
    }
}

#[test]
fn should_reject_page_entries_without_a_retained_history() {
    let fixture = Fixture::new();
    for filter in [
        DocumentHistoryFilter::StartAtTime(0),
        DocumentHistoryFilter::StartAtRevision(1),
    ] {
        let query = fixture.query(filter.clone(), None);
        fixture
            .decode(&query, vec![])
            .expect("omitted exact metadata keys authenticate an absent history");
        let message = if matches!(filter, DocumentHistoryFilter::StartAtTime(_)) {
            "history page exceeds its limit or revision count"
        } else {
            "history page length disagrees with its revision count"
        };
        assert_corrupt(fixture.decode(&query, vec![fixture.entry_row(1)]), message);
    }
}

#[test]
fn should_reject_a_duplicate_history_tree_echo() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    rows.push(rows[1].clone());
    assert_corrupt(fixture.decode(&query, rows), "duplicate history metadata");
}

#[test]
fn should_reject_a_duplicate_current_pointer() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    rows.push(rows[0].clone());
    assert_corrupt(
        fixture.decode(&query, rows),
        "duplicate history current pointer",
    );
}

#[test]
fn should_reject_metadata_keys_that_do_not_match_the_document() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    for index in [0, 1] {
        let mut rows = fixture.rows(&[1, 2, 3]);
        rows[index].1[0] ^= 1;
        assert_corrupt(
            fixture.decode(&query, rows),
            "history metadata key does not match the document",
        );
    }
}

#[test]
fn should_reject_an_active_document_with_zero_retained_revisions() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[]);
    rows[1].2 = Some(Element::ProvableCountTree(None, 0, None));
    assert_corrupt(
        fixture.decode(&query, rows),
        "current document and retained history disagree",
    );
}

#[test]
fn should_reject_a_current_pointer_to_another_document() {
    let mut fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    let mut other_id = query.document_id;
    other_id[0] ^= 1;
    fixture.document.set_id(other_id.into());
    rows[0].2 = Some(fixture.document_element(3));
    assert_corrupt(
        fixture.decode(&query, rows),
        "history current document id does not match the query",
    );
}

#[test]
fn should_reject_a_current_pointer_that_is_not_an_item() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    rows[0].2 = Some(Element::empty_tree());
    assert_corrupt(
        fixture.decode(&query, rows),
        "history current pointer did not resolve to a document",
    );
}

#[test]
fn should_reject_a_present_reserved_lifecycle_record() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    rows.push(fixture.metadata_row(1, Some(Element::new_item(vec![1]))));
    assert_corrupt(
        fixture.decode(&query, rows),
        "unexpected lifecycle record or history metadata path",
    );
}

#[test]
fn should_reject_a_history_echo_without_an_authenticated_count() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    for element in [
        Element::empty_tree(),
        Element::new_item(vec![1]),
        Element::CountTree(Some(vec![1]), 3, None),
        Element::ProvableCountSumTree(Some(vec![1]), 3, 0, None),
    ] {
        let mut rows = fixture.rows(&[1, 2, 3]);
        rows[1] = fixture.metadata_row(DOCUMENT_HISTORY_TREE_KEY, Some(element));
        assert_corrupt(
            fixture.decode(&query, rows),
            "history metadata is not a provable count tree",
        );
    }
}

#[test]
fn should_reject_an_absent_page_entry_element() {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtRevision(2), Some(10));
    let mut rows = fixture.rows(&[2, 3]);
    rows[2].2 = None;
    assert_corrupt(
        fixture.decode(&query, rows),
        "history entry contains an absent element",
    );
}

fn assert_rejects_absent_metadata(branch: u8) {
    let fixture = Fixture::new();
    let query = fixture.query(DocumentHistoryFilter::StartAtTime(0), None);
    let mut rows = fixture.rows(&[1, 2, 3]);
    rows.push(fixture.metadata_row(branch, None));
    assert_corrupt(
        fixture.decode(&query, rows),
        "history metadata contains an absent element",
    );
}

#[test]
fn should_reject_an_absent_current_pointer_metadata_element() {
    assert_rejects_absent_metadata(0);
}

#[test]
fn should_reject_an_absent_reserved_metadata_element() {
    assert_rejects_absent_metadata(1);
}

#[test]
fn should_reject_an_absent_history_metadata_element() {
    assert_rejects_absent_metadata(DOCUMENT_HISTORY_TREE_KEY);
}
