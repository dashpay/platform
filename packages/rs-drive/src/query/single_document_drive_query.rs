use crate::drive::document::paths::contract_document_type_path_vec;
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
        match self.contested_status {
            SingleDocumentDriveQueryContestedStatus::NotContested => {
                Ok(self.construct_non_contested_path_query(true))
            }
            SingleDocumentDriveQueryContestedStatus::MaybeContested => {
                let non_contested = self.construct_non_contested_path_query(true);
                let contested = self.construct_contested_path_query(true);
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
    fn construct_non_contested_path_query(&self, with_limit_1: bool) -> PathQuery {
        // First we should get the overall document_type_path
        let mut path =
            contract_document_type_path_vec(&self.contract_id, self.document_type_name.as_str());

        path.push(vec![0]);

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        if self.document_type_keeps_history {
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

        PathQuery::new(path, SizedQuery::new(query, limit, None))
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
