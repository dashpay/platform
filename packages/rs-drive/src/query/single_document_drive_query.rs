mod v0;
mod v1;

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
    /// version, and one with a block time for the newest version stored at or before that
    /// time. A lookup at a block time answers so from protocol version 15: before it, a time
    /// before the document was first stored answers with the latest version, and no proof of
    /// such a lookup verifies.
    ///
    /// # Parameters
    ///
    /// - `platform_version`: The platform version, whose
    ///   `drive.methods.document.query.single_document_path_query` selects the query built.
    ///
    /// # Returns
    ///
    /// The path query, or an error when the version is unknown or the two places a document
    /// that may be contested is looked for in can not be merged into one query.
    pub fn construct_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
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
    use dpp::decision_proposal::{
        DECISION_PROPOSALS_CONTRACT_ID, DECISION_PROPOSAL_FUND_NAME, PROPOSAL_DOCUMENT_TYPE_NAME,
    };
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
    use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
    use grovedb::SizedQuery;

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

    #[test]
    fn should_build_the_query_the_protocol_version_before_built_except_at_a_block_time() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        for query in every_query().filter(|query| !looks_in_history_at_block_time(query)) {
            assert_eq!(
                query
                    .construct_path_query(PlatformVersion::latest())
                    .expect("expected to build the query"),
                query
                    .construct_path_query(protocol_version_14)
                    .expect("expected to build the query at protocol version 14"),
                "{query:?}"
            );
        }
    }

    #[test]
    fn should_leave_the_lookup_at_a_block_time_of_protocol_version_14_as_it_was() {
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        // Every key up to the block time, the pointer to the latest version at `[0]` included
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
                        &query.construct_non_contested_path_query(true),
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
                .construct_path_query(PlatformVersion::latest())
                .expect("expected to build the query"),
                history_lookup(versions),
                "block time: {block_time_ms}"
            );
        }
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
