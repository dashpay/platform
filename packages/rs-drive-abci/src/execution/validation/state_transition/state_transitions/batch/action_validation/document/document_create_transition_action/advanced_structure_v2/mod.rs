use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::decision_proposal::DecisionProposalFundMismatchError;
use dpp::consensus::state::decision_proposal::DecisionProposalEndDateOutOfRangeError;
use dpp::dashcore::Network;
use dpp::decision_proposal::{
    decision_proposal_required_fund, DecisionProposalVotePoll, DECISION_PROPOSAL_FUND_NAME,
};
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::advanced_structure_v1::DocumentCreateTransitionActionStructureValidationV1;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStructureValidationV2 {
    fn validate_structure_v2(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionStructureValidationV2 for DocumentCreateTransitionAction {
    /// Version 1, then a decision proposal (protocol version 15) must state exactly the
    /// decision proposal fund under the proposal's name, its rule must make a yes/no poll within
    /// the system limits, and it must end one to four weeks after the block. All three are on
    /// the transition, judged against the block as v1's property rules are, so check_tx refuses
    /// what a block would.
    fn validate_structure_v2(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self.validate_structure_v1(owner_id, block_info, network, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        // -->> Introduced in V2 <<--
        let Some(decision_proposal) = self.decision_proposal() else {
            return Ok(result);
        };
        let document_id = self.base().id();

        // The fund prefunds the poll's votes. Stated exactly, so a proposer signs what it pays.
        let required_fund = decision_proposal_required_fund(platform_version);
        let states_the_fund = matches!(
            &decision_proposal.stated_fund,
            Some((name, amount)) if name == DECISION_PROPOSAL_FUND_NAME && *amount == required_fund
        );
        if !states_the_fund {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DecisionProposalFundMismatchError::new(
                    document_id,
                    required_fund,
                    decision_proposal.stated_fund.clone(),
                )
                .into(),
            ));
        }

        // The rule: the schema validation in v1 has held every property to its type, so what is
        // left is whether the rule makes a poll the system limits allow
        let vote_poll =
            DecisionProposalVotePoll::from_proposal(document_id, self.data(), platform_version)
                .map_err(Error::Protocol)?;
        if !vote_poll.is_valid() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                vote_poll.errors,
            ));
        }
        let vote_poll = vote_poll.into_data().map_err(Error::Protocol)?;

        // A poll ending too soon closes before masternode operators see it, and one ending too
        // late holds its fund and trees for as long: from one to four weeks after this block
        let end_date_window = YesNoVotePoll::end_date_window(block_info.time_ms, platform_version)
            .map_err(Error::Protocol)?;
        if !end_date_window.contains(&vote_poll.end_date) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DecisionProposalEndDateOutOfRangeError::new(
                    document_id,
                    vote_poll.end_date,
                    *end_date_window.start(),
                    *end_date_window.end(),
                )
                .into(),
            ));
        }
        Ok(result)
        // -->> End Introduced in V2 <<--
    }
}
