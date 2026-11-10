mod v0;
mod v1;

use crate::query::primary_key_path_query::primary_key_path_query_target;

use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::decision_proposal::is_decision_proposal;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use grovedb::PathQuery;
use platform_version::version::PlatformVersion;
use platform_version::TryFromPlatformVersioned;

/// The expected contested status of a document
/// Drives stores the document in either the not contested location (most of the time)
/// Or a temporary contested area while the contest is ongoing
#[derive(Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum SingleDocumentDriveQueryContestedStatus {
    /// The document was not contested by the system.
    NotContested = 0,
    /// We don't know if the document was contested by the system, or we are not sure if the contest
    /// is already over or not.
    MaybeContested = 1,
    /// We know that the document was contested by the system and the contest is not over.
    Contested = 2,
}

impl SingleDocumentDriveQueryContestedStatus {
    /// Where the document of `document_transition` is stored once the transition executes,
    /// which is where its proof looks. A create stating a fund opens a contest, and its
    /// document waits in the contest's area. A decision proposal (protocol version 15) states a
    /// fund too, but the fund pays the yes/no poll it opens, and the proposal is stored as any
    /// document is.
    pub fn of_document_transition(document_transition: &DocumentTransition) -> Self {
        match document_transition {
            DocumentTransition::Create(create_transition)
                if create_transition.prefunded_voting_balance().is_some()
                    && !is_decision_proposal(
                        &document_transition.data_contract_id(),
                        document_transition.document_type_name(),
                    ) =>
            {
                SingleDocumentDriveQueryContestedStatus::Contested
            }
            _ => SingleDocumentDriveQueryContestedStatus::NotContested,
        }
    }
}

impl TryFrom<i32> for SingleDocumentDriveQueryContestedStatus {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SingleDocumentDriveQueryContestedStatus::NotContested),
            1 => Ok(SingleDocumentDriveQueryContestedStatus::MaybeContested),
            2 => Ok(SingleDocumentDriveQueryContestedStatus::Contested),
            n => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "unsupported contested status {}, only 0, 1 and 2 are supported",
                n
            )))),
        }
    }
}

/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct SingleDocumentDriveQuery {
    ///DataContract
    pub contract_id: [u8; 32],
    /// Document type
    pub document_type_name: String,
    /// Document type keeps history
    pub document_type_keeps_history: bool,
    /// Document
    pub document_id: [u8; 32],
    /// Block time
    pub block_time_ms: Option<u64>,
    /// True if the document might have gone to a contested resolution
    pub contested_status: SingleDocumentDriveQueryContestedStatus,
}

impl SingleDocumentDriveQuery {
    /// The query of document `document_id` of `document_type_name` on `contract_id` as it
    /// stands now, outside any contest: the query a moderator's deletion without a record and
    /// a moderator's field change are proved by. The prover and the verifier both build it
    /// here, so the two can not drift apart.
    pub fn latest_not_contested(
        contract_id: [u8; 32],
        document_type_name: String,
        document_type_keeps_history: bool,
        document_id: [u8; 32],
    ) -> Self {
        SingleDocumentDriveQuery {
            contract_id,
            document_type_name,
            document_type_keeps_history,
            document_id,
            block_time_ms: None,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        }
    }

    /// The GroveDB query that looks the document up, which the prover and the verifier both
    /// build here.
    ///
    /// In a document type that keeps history, a query with no block time asks for the latest
    /// version, and one with a block time for the newest retained revision at or before that
    /// time, taking the highest revision when times match. This behavior is selected from
    /// protocol version 15. Before it, a time before the document was first stored answers
    /// with the latest version, and no proof of such a lookup verifies. From protocol version
    /// 15 a document with history that may be contested is refused at a block time, as a
    /// contest's area keeps no history.
    ///
    /// # Parameters
    ///
    /// - `platform_version`: The platform version, whose
    ///   `drive.methods.document.query.single_document_path_query` selects the query built.
    ///
    /// # Returns
    ///
    /// The path query, or an error when the version is unknown, the lookup is refused, or the
    /// two places a document that may be contested is looked for in can not be merged into one
    /// query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        primary_key_path_query_target(platform_version)?;
        match platform_version
            .drive
            .methods
            .document
            .query
            .single_document_path_query
        {
            0 => self.construct_path_query_v0(platform_version),
            1 => self.construct_path_query_v1(platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "SingleDocumentDriveQuery::construct_path_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

impl TryFromPlatformVersioned<SingleDocumentDriveQuery> for PathQuery {
    type Error = Error;
    fn try_from_platform_versioned(
        value: SingleDocumentDriveQuery,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        value.construct_path_query(platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::query::Query;
    use crate::util::common::encode::encode_u64;
    #[cfg(feature = "full")]
    use crate::util::test_helpers::setup::setup_drive;
    #[cfg(feature = "full")]
    use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
    #[cfg(feature = "full")]
    use grovedb::{Element, GroveDb};
    #[cfg(feature = "full")]
    use grovedb_version::version::{v4::GROVE_V4, v5::GROVE_V5};
    #[cfg(feature = "full")]
    use std::collections::BTreeSet;
    use dpp::decision_proposal::{
        DECISION_PROPOSALS_CONTRACT_ID, DECISION_PROPOSAL_FUND_NAME, PROPOSAL_DOCUMENT_TYPE_NAME,
    };
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
    use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
    use grovedb::SizedQuery;

    fn keep_history_query(block_time_ms: Option<u64>) -> SingleDocumentDriveQuery {
        SingleDocumentDriveQuery {
            contract_id: [1; 32],
            document_type_name: "note".to_string(),
            document_type_keeps_history: true,
            document_id: [2; 32],
            block_time_ms,
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        }
    }

    #[test]
    fn should_select_keep_history_primary_path_by_query_version() {
        let query = keep_history_query(None);
        let legacy = query
            .construct_path_query(PlatformVersion::get(14).expect("protocol 14"))
            .expect("legacy query");
        assert_eq!(
            legacy.query.query.default_subquery_branch.subquery_path,
            Some(vec![vec![0]])
        );

        let current = query
            .construct_path_query(PlatformVersion::get(15).expect("protocol 15"))
            .expect("protocol 15 query");
        assert_eq!(
            current.query.query.default_subquery_branch.subquery_path,
            None
        );

        let legacy_point_in_time = keep_history_query(Some(1000))
            .construct_path_query(PlatformVersion::get(14).expect("protocol 14"))
            .expect("legacy point-in-time query");
        assert!(legacy_point_in_time
            .query
            .query
            .default_subquery_branch
            .subquery
            .is_some());
        keep_history_query(Some(1000))
            .construct_path_query(PlatformVersion::get(15).expect("protocol 15"))
            .expect("single-document temporal history is available after activation");
    }

    #[test]
    fn should_merge_maybe_contested_legacy_history_for_a_protocol_15_caller() {
        let mut caller = PlatformVersion::get(15).expect("protocol 15").clone();
        caller
            .drive
            .methods
            .document
            .document_history_storage_layout = 0;
        let mut query = keep_history_query(None);
        query.contested_status = SingleDocumentDriveQueryContestedStatus::MaybeContested;
        let actual = query
            .construct_path_query(&caller)
            .expect("legacy history query merges");
        let expected = query
            .construct_path_query(PlatformVersion::get(14).expect("protocol 14"))
            .expect("released query merges");
        assert_eq!(actual, expected);
    }

    #[cfg(feature = "full")]
    #[test]
    fn should_preserve_single_document_merge_bytes_and_results_across_layouts() {
        for layout in [0, 1] {
            for keeps_history in [false, true] {
                for present in [false, true] {
                    let mut caller = PlatformVersion::latest().clone();
                    caller
                        .drive
                        .methods
                        .document
                        .document_history_storage_layout = layout;
                    let mut previous = caller.clone();
                    previous.drive.grove_version = GROVE_V4;
                    let drive = setup_drive(None);
                    let mut query = keep_history_query(None);
                    query.document_type_keeps_history = keeps_history;
                    let ordinary = query
                        .construct_non_contested_path_query(true, &caller)
                        .unwrap();
                    let contested = query.construct_contested_path_query(true);
                    let mut created = BTreeSet::new();
                    for path in [&ordinary.path, &contested.path] {
                        for depth in 0..path.len() {
                            if created.insert(path[..=depth].to_vec()) {
                                drive
                                    .grove
                                    .insert(
                                        &path[..depth],
                                        &path[depth],
                                        Element::empty_tree(),
                                        None,
                                        None,
                                        &GROVE_V5,
                                    )
                                    .unwrap()
                                    .expect("selection subtree");
                            }
                        }
                    }
                    if present {
                        let mut document_path = ordinary.path.clone();
                        let key = if keeps_history && layout == 0 {
                            drive
                                .grove
                                .insert(
                                    document_path.as_slice(),
                                    &query.document_id,
                                    Element::empty_tree(),
                                    None,
                                    None,
                                    &GROVE_V5,
                                )
                                .unwrap()
                                .expect("legacy document subtree");
                            document_path.push(query.document_id.to_vec());
                            vec![0]
                        } else {
                            query.document_id.to_vec()
                        };
                        drive
                            .grove
                            .insert(
                                document_path.as_slice(),
                                &key,
                                Element::new_item(vec![7]),
                                None,
                                None,
                                &GROVE_V5,
                            )
                            .unwrap()
                            .expect("ordinary document");
                        drive
                            .grove
                            .insert(
                                contested.path.as_slice(),
                                &query.document_id,
                                Element::new_item(vec![8]),
                                None,
                                None,
                                &GROVE_V5,
                            )
                            .unwrap()
                            .expect("contested document");
                    }
                    for status in [
                        SingleDocumentDriveQueryContestedStatus::NotContested,
                        SingleDocumentDriveQueryContestedStatus::Contested,
                        SingleDocumentDriveQueryContestedStatus::MaybeContested,
                    ] {
                        query.contested_status = status;
                        let old = query.construct_path_query(&previous).expect("V4 query");
                        let new = query.construct_path_query(&caller).expect("V5 query");
                        assert_eq!(
                            old, new,
                            "layout {layout}, history {keeps_history}, present {present}"
                        );
                        let config = bincode::config::standard().with_big_endian();
                        assert_eq!(
                            bincode::encode_to_vec(&old, config).unwrap(),
                            bincode::encode_to_vec(&new, config).unwrap()
                        );
                        let read = |selection: &PathQuery, grove_version| {
                            drive
                                .grove
                                .query_raw(
                                    selection,
                                    false,
                                    true,
                                    true,
                                    QueryPathKeyElementTrioResultType,
                                    None,
                                    grove_version,
                                )
                                .unwrap()
                                .expect("document selection")
                                .0
                                .to_path_key_elements()
                        };
                        let expected = read(&old, &GROVE_V4);
                        assert_eq!(read(&new, &GROVE_V5), expected);
                        assert_eq!(
                            expected.len(),
                            if present {
                                if query.contested_status
                                    == SingleDocumentDriveQueryContestedStatus::MaybeContested
                                {
                                    2
                                } else {
                                    1
                                }
                            } else {
                                0
                            }
                        );
                        for (selection, grove_version) in [(&old, &GROVE_V4), (&new, &GROVE_V5)] {
                            let proof = drive
                                .grove
                                .prove_query(selection, None, grove_version)
                                .unwrap()
                                .expect("document proof");
                            let (root, rows) =
                                GroveDb::verify_query(&proof, selection, grove_version)
                                    .expect("verified document proof");
                            assert_eq!(
                                root,
                                drive.grove.root_hash(None, grove_version).unwrap().unwrap()
                            );
                            let (current_root, current_rows) =
                                GroveDb::verify_query(&proof, &new, &GROVE_V5)
                                    .expect("current verifier accepts the producer's proof");
                            assert_eq!((current_root, &current_rows), (root, &rows));
                            assert_eq!(
                                rows.into_iter()
                                    .filter(|(_, _, element)| element.is_some())
                                    .count(),
                                expected.len()
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn should_reject_unknown_primary_key_path_query_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .document
            .document_history_storage_layout = 2;

        let mut query = keep_history_query(None);
        query.document_type_keeps_history = false;
        let error = query
            .construct_path_query(&platform_version)
            .expect_err("unknown path-query versions must fail closed");
        assert!(matches!(
            error,
            Error::Drive(DriveError::UnknownVersionMismatch {
                method,
                known_versions,
                received: 2,
            }) if method == "document_history_storage_layout" && known_versions == vec![0, 1]
        ));
    }

    fn create(
        contract_id: Identifier,
        document_type_name: &str,
        stated_fund: Option<(String, Credits)>,
    ) -> DocumentTransition {
        let mut create = DocumentCreateTransition::default();
        create.base_mut().set_data_contract_id(contract_id);
        create
            .base_mut()
            .set_document_type_name(document_type_name.to_string());
        *create.prefunded_voting_balances_mut() = stated_fund;
        DocumentTransition::Create(create)
    }

    fn single_document(
        contested_status: SingleDocumentDriveQueryContestedStatus,
        document_type_keeps_history: bool,
        block_time_ms: Option<u64>,
    ) -> SingleDocumentDriveQuery {
        SingleDocumentDriveQuery {
            contract_id: [7; 32],
            document_type_name: "card".to_string(),
            document_type_keeps_history,
            document_id: [9; 32],
            block_time_ms,
            contested_status,
        }
    }

    /// The query of the document among the documents of its type, in a type that keeps
    /// history, with `versions` as the lookup in the document's history.
    fn history_lookup(versions: Query) -> PathQuery {
        let mut path = contract_document_type_path_vec(&[7; 32], "card");
        path.push(vec![0]);
        let mut query = Query::new();
        query.insert_key(vec![9; 32]);
        query.set_subquery(versions);
        PathQuery::new(path, SizedQuery::new(query, Some(1), None))
    }

    /// Whether generation 1 builds another query than generation 0: a lookup at a block time
    /// among the documents of a type that keeps history
    fn looks_in_history_at_block_time(query: &SingleDocumentDriveQuery) -> bool {
        query.document_type_keeps_history
            && query.block_time_ms.is_some()
            && query.contested_status != SingleDocumentDriveQueryContestedStatus::Contested
    }

    fn every_query() -> impl Iterator<Item = SingleDocumentDriveQuery> {
        [
            SingleDocumentDriveQueryContestedStatus::NotContested,
            SingleDocumentDriveQueryContestedStatus::MaybeContested,
            SingleDocumentDriveQueryContestedStatus::Contested,
        ]
        .into_iter()
        .flat_map(|contested_status| {
            [false, true]
                .into_iter()
                .flat_map(move |document_type_keeps_history| {
                    let contested_status = contested_status.clone();
                    [None, Some(1500)].into_iter().map(move |block_time_ms| {
                        single_document(
                            contested_status.clone(),
                            document_type_keeps_history,
                            block_time_ms,
                        )
                    })
                })
        })
    }

    /// Generation 1 changes one lookup only, at a block time in a document type that keeps
    /// history. Every other query is the one protocol version 14 built, the capped one of a
    /// document that may be contested included, so a proof keeps its shape from one version to
    /// the next
    #[test]
    fn should_build_the_query_the_protocol_version_before_built_except_at_a_block_time() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        let caller = legacy_layout_caller();
        for query in every_query().filter(|query| !looks_in_history_at_block_time(query)) {
            assert_eq!(
                query
                    .construct_path_query(&caller)
                    .expect("expected to build the query"),
                query
                    .construct_path_query(protocol_version_14)
                    .expect("expected to build the query at protocol version 14"),
                "{query:?}"
            );
            let current = query
                .construct_path_query(PlatformVersion::latest())
                .unwrap();
            let legacy = query.construct_path_query(&caller).unwrap();
            if query.document_type_keeps_history
                && query.contested_status != SingleDocumentDriveQueryContestedStatus::Contested
            {
                assert_ne!(current, legacy, "current documents use primary pointers");
            } else {
                assert_eq!(current, legacy);
            }
        }
    }

    /// Protocol version 14 selects generation 0, whose range takes every key up to the block
    /// time, the pointer to the latest version at `[0]` included
    #[test]
    fn should_leave_the_lookup_at_a_block_time_of_protocol_version_14_as_it_was() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        let mut versions = Query::new_with_direction(false);
        versions.insert_range_to_inclusive(..=encode_u64(1500));
        let not_contested = history_lookup(versions);

        assert_eq!(
            single_document(
                SingleDocumentDriveQueryContestedStatus::NotContested,
                true,
                Some(1500),
            )
            .construct_path_query(protocol_version_14)
            .expect("expected to build the query"),
            not_contested
        );
    }

    /// Protocol version 14 selects generation 0 for a document that may be contested too: it
    /// builds the plain merge of the two lookups, with and without a block time
    #[test]
    fn should_leave_the_maybe_contested_query_of_protocol_version_14_as_it_was() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        for block_time_ms in [None, Some(1500)] {
            let query = single_document(
                SingleDocumentDriveQueryContestedStatus::MaybeContested,
                true,
                block_time_ms,
            );

            assert_eq!(
                query
                    .construct_path_query(protocol_version_14)
                    .expect("expected to build the query"),
                PathQuery::merge(
                    vec![
                        &query
                            .construct_non_contested_path_query(true, protocol_version_14)
                            .unwrap(),
                        &query.construct_contested_path_query(true),
                    ],
                    &protocol_version_14.drive.grove_version,
                )
                .expect("expected to merge the two lookups"),
                "block time: {block_time_ms:?}"
            );
        }
    }

    #[test]
    fn should_stop_a_lookup_at_a_block_time_above_the_pointer_to_the_latest_version() {
        let caller = legacy_layout_caller();
        for (block_time_ms, last_time_ms) in [
            (1500, 1500),
            // A time the history can not order asks for the last one it can
            (u64::MAX, i64::MAX as u64),
        ] {
            let mut versions = Query::new_with_direction(false);
            versions.insert_range_after_to_inclusive(vec![0]..=encode_u64(last_time_ms));

            assert_eq!(
                single_document(
                    SingleDocumentDriveQueryContestedStatus::NotContested,
                    true,
                    Some(block_time_ms),
                )
                .construct_path_query(&caller)
                .expect("expected to build the query"),
                history_lookup(versions),
                "block time: {block_time_ms}"
            );
        }
    }

    /// A contest's area keeps no history, so a document with history that may be contested
    /// can not be looked up at a block time. Without history the time is ignored, as in every
    /// status
    #[test]
    fn should_refuse_a_lookup_at_a_block_time_of_a_document_with_history_that_may_be_contested() {
        for caller in [legacy_layout_caller(), PlatformVersion::latest().clone()] {
            let result = single_document(
                SingleDocumentDriveQueryContestedStatus::MaybeContested,
                true,
                Some(1500),
            )
            .construct_path_query(&caller);

            assert!(
                matches!(&result, Err(Error::Query(QuerySyntaxError::Unsupported(message)))
                if message == "a document with history that may be contested can not be looked up at a block time"),
                "{result:?}"
            );
            assert!(single_document(
                SingleDocumentDriveQueryContestedStatus::MaybeContested,
                false,
                Some(1500),
            )
            .construct_path_query(&caller)
            .is_ok());
        }
    }

    fn legacy_layout_caller() -> PlatformVersion {
        let mut caller = PlatformVersion::latest().clone();
        caller
            .drive
            .methods
            .document
            .document_history_storage_layout = 0;
        caller
    }

    #[test]
    fn should_bound_temporal_composite_history_in_descending_order() {
        for time in [1500, u64::MAX] {
            let mut upper = encode_u64(time.min(i64::MAX as u64));
            upper.extend([0xff; 8]);
            let mut versions = Query::new_with_direction(false);
            versions.insert_range_inclusive(encode_u64(0)..=upper);
            let mut expected = history_lookup(versions);
            expected.path[4] = vec![2];
            assert!(!expected.query.query.add_parent_tree_on_subquery);
            assert_eq!(
                single_document(
                    SingleDocumentDriveQueryContestedStatus::NotContested,
                    true,
                    Some(time)
                )
                .construct_path_query(PlatformVersion::latest())
                .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn should_keep_generation_zero_layout_one_temporal_reads_refused() {
        let mut caller = PlatformVersion::get(14).unwrap().clone();
        caller
            .drive
            .methods
            .document
            .document_history_storage_layout = 1;
        let result = keep_history_query(Some(1000)).construct_path_query(&caller);
        assert!(
            matches!(result, Err(Error::Query(QuerySyntaxError::Unsupported(message)))
            if message == "point-in-time reads are unavailable for history-keeping document types")
        );
    }

    #[test]
    fn should_refuse_an_unknown_version_of_the_query() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .document
            .query
            .single_document_path_query = 255;

        let result = single_document(
            SingleDocumentDriveQueryContestedStatus::NotContested,
            false,
            None,
        )
        .construct_path_query(&platform_version);

        assert!(
            matches!(
                &result,
                Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received }))
                    if method == "SingleDocumentDriveQuery::construct_path_query"
                        && known_versions == &vec![0, 1]
                        && *received == 255
            ),
            "{result:?}"
        );
    }

    /// A create stating a contest fund is looked for in the contest's area, while a decision
    /// proposal, which states the decision proposal fund, is stored and looked for as any
    /// document
    #[test]
    fn should_look_for_a_decision_proposal_among_documents_and_a_contested_create_in_its_contest() {
        let contract_id = Identifier::new([7; 32]);
        assert_eq!(
            SingleDocumentDriveQueryContestedStatus::of_document_transition(&create(
                contract_id,
                "domain",
                Some(("parentNameAndLabel".to_string(), 20_000_000_000)),
            )),
            SingleDocumentDriveQueryContestedStatus::Contested
        );
        assert_eq!(
            SingleDocumentDriveQueryContestedStatus::of_document_transition(&create(
                contract_id,
                "domain",
                None,
            )),
            SingleDocumentDriveQueryContestedStatus::NotContested
        );
        assert_eq!(
            SingleDocumentDriveQueryContestedStatus::of_document_transition(&create(
                DECISION_PROPOSALS_CONTRACT_ID,
                PROPOSAL_DOCUMENT_TYPE_NAME,
                Some((DECISION_PROPOSAL_FUND_NAME.to_string(), 50_000_000_000)),
            )),
            SingleDocumentDriveQueryContestedStatus::NotContested
        );
    }
}
