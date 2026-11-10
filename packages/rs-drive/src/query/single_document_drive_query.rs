use crate::drive::document::paths::contract_document_type_path_vec;
use crate::query::primary_key_path_query::{
    primary_key_path_query_target, PrimaryKeyPathQueryTarget,
};
use crate::util::common::encode::encode_u64;

use crate::drive::votes;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::Query;
use dpp::decision_proposal::is_decision_proposal;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use grovedb::{PathQuery, SizedQuery};
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

    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.document_type_keeps_history
            && self.block_time_ms.is_some()
            && primary_key_path_query_target(platform_version)?
                == PrimaryKeyPathQueryTarget::CurrentDocument
        {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "point-in-time reads are unavailable for history-keeping document types"
                    .to_string(),
            )));
        }
        match self.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => {
                self.construct_non_contested_path_query(true, platform_version)
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => {
                let mut non_contested =
                    self.construct_non_contested_path_query(true, platform_version)?;
                let mut contested = self.construct_contested_path_query(true);
                if platform_version
                    .drive
                    .grove_version
                    .grovedb_versions
                    .path_query_methods
                    .merge
                    == 2
                {
                    // A block-time lookup through the legacy history subtree also reaches
                    // its latest-version pointer. Its range cannot authenticate an earlier
                    // absence, so this merged lookup refuses it.
                    if self.document_type_keeps_history && self.block_time_ms.is_some() {
                        return Err(Error::Query(QuerySyntaxError::Unsupported(
                            "a document with history that may be contested can not be looked up at a block time".to_string(),
                        )));
                    }
                    // Each branch selects one document, so turning its global limit into an
                    // instance cap produces the same merged bytes as the earlier limit lift.
                    for selection in [&mut non_contested, &mut contested] {
                        if let Some(global_limit) = selection.query.limit.take() {
                            selection.query.query.limit = Some(
                                selection
                                    .query
                                    .query
                                    .limit
                                    .map_or(global_limit, |limit| limit.min(global_limit)),
                            );
                        }
                    }
                }
                PathQuery::merge(
                    vec![&non_contested, &contested],
                    &platform_version.drive.grove_version,
                )
                .map_err(Error::from)
            }
            SingleDocumentDriveQueryContestedStatus::Contested => {
                Ok(self.construct_contested_path_query(true))
            }
        }
    }

    /// Operations to construct the normal path query.
    fn construct_non_contested_path_query(
        &self,
        with_limit_1: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let uses_current_document = primary_key_path_query_target(platform_version)?
            == PrimaryKeyPathQueryTarget::CurrentDocument;
        // First we should get the overall document_type_path
        let mut path =
            contract_document_type_path_vec(&self.contract_id, self.document_type_name.as_str());

        path.push(vec![0]);

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        if self.document_type_keeps_history && !uses_current_document {
            // if the documents keep history then we should insert a subquery
            if let Some(block_time) = self.block_time_ms {
                let encoded_block_time = encode_u64(block_time);
                let mut sub_query = Query::new_with_direction(false);
                sub_query.insert_range_to_inclusive(..=encoded_block_time);
                query.set_subquery(sub_query);
            } else {
                query.set_subquery_key(vec![0]);
            }
        }

        let limit = if with_limit_1 { Some(1) } else { None };

        Ok(PathQuery::new(path, SizedQuery::new(query, limit, None)))
    }

    /// Operations to construct the contested path query.
    fn construct_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
        // First we should get the overall document_type_path
        let path = votes::paths::vote_contested_resource_contract_documents_storage_path_vec(
            &self.contract_id,
            self.document_type_name.as_str(),
        );

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        let limit = if with_limit_1 { Some(1) } else { None };

        PathQuery::new(path, SizedQuery::new(query, limit, None))
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
    use crate::error::drive::DriveError;
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
        assert!(matches!(
            keep_history_query(Some(1000))
                .construct_path_query(PlatformVersion::get(15).expect("protocol 15")),
            Err(Error::Query(QuerySyntaxError::Unsupported(message)))
                if message == "point-in-time reads are unavailable for history-keeping document types"
        ));
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

    use dpp::decision_proposal::{
        DECISION_PROPOSALS_CONTRACT_ID, DECISION_PROPOSAL_FUND_NAME, PROPOSAL_DOCUMENT_TYPE_NAME,
    };
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
    use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;

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

    fn maybe_contested(
        document_type_keeps_history: bool,
        block_time_ms: Option<u64>,
    ) -> SingleDocumentDriveQuery {
        SingleDocumentDriveQuery {
            contract_id: [7; 32],
            document_type_name: "card".to_string(),
            document_type_keeps_history,
            document_id: [9; 32],
            block_time_ms,
            contested_status: SingleDocumentDriveQueryContestedStatus::MaybeContested,
        }
    }

    /// Lowering the cap preserves the released query for the legacy history layout.
    #[test]
    fn should_build_the_maybe_contested_query_the_protocol_version_before_built() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        for document_type_keeps_history in [false, true] {
            let query = maybe_contested(document_type_keeps_history, None);

            let mut caller = PlatformVersion::latest().clone();
            caller
                .drive
                .methods
                .document
                .document_history_storage_layout = 0;
            assert_eq!(
                query
                    .construct_path_query(&caller)
                    .expect("expected to build the query"),
                query
                    .construct_path_query(protocol_version_14)
                    .expect("expected to build the query at protocol version 14"),
                "keeps history: {document_type_keeps_history}"
            );
            let current = query
                .construct_path_query(PlatformVersion::latest())
                .unwrap();
            let legacy = query.construct_path_query(&caller).unwrap();
            if document_type_keeps_history {
                assert_ne!(current, legacy, "current documents use primary pointers");
            } else {
                assert_eq!(current, legacy);
            }
        }
    }

    /// Protocol version 14 does not enter the new branch: it builds the plain merge of the two
    /// lookups, with and without a block time
    #[test]
    fn should_leave_the_maybe_contested_query_of_protocol_version_14_as_it_was() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        for block_time_ms in [None, Some(1500)] {
            let query = maybe_contested(true, block_time_ms);

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
    fn should_refuse_a_maybe_contested_lookup_at_a_block_time() {
        let result =
            maybe_contested(true, Some(1500)).construct_path_query(PlatformVersion::latest());

        assert!(
            matches!(result, Err(Error::Query(QuerySyntaxError::Unsupported(_)))),
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
