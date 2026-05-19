/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::state_transition_action::identity::masternode_vote::v0::{
    MasternodeVoteTransitionActionV0, PreviousVoteCount,
};
use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

/// action
#[derive(Debug, Clone, From)]
pub enum MasternodeVoteTransitionAction {
    /// v0
    V0(MasternodeVoteTransitionActionV0),
}

impl MasternodeVoteTransitionAction {
    /// the pro tx hash identifier of the masternode
    pub fn pro_tx_hash(&self) -> Identifier {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.pro_tx_hash,
        }
    }

    /// the voter identity id
    pub fn voter_identity_id(&self) -> Identifier {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.voter_identity_id,
        }
    }

    /// the masternode list state based voting address
    pub fn voting_address(&self) -> [u8; 20] {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.voting_address,
        }
    }

    /// Resource votes
    pub fn vote_ref(&self) -> &ResolvedVote {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => &transition.vote,
        }
    }

    /// Resource votes as owned
    pub fn vote_owned(self) -> ResolvedVote {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.vote,
        }
    }

    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// Vote strength
    pub fn vote_strength(&self) -> u8 {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => transition.vote_strength,
        }
    }

    /// The previous resource vote choice that needs to be removed
    pub fn take_previous_resource_vote_choice_to_remove(
        &mut self,
    ) -> Option<(ResourceVoteChoice, PreviousVoteCount)> {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => {
                transition.previous_resource_vote_choice_to_remove.take()
            }
        }
    }

    /// The previous resource vote choice that needs to be removed
    pub fn previous_resource_vote_choice_to_remove(
        &self,
    ) -> &Option<(ResourceVoteChoice, PreviousVoteCount)> {
        match self {
            MasternodeVoteTransitionAction::V0(transition) => {
                &transition.previous_resource_vote_choice_to_remove
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
    use crate::drive::votes::resolved::votes::resolved_resource_vote::v0::ResolvedResourceVoteV0;
    use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
    use crate::drive::votes::resolved::votes::ResolvedVote;
    use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use platform_version::version::PlatformVersion;

    fn make_resolved_vote() -> ResolvedVote {
        let platform_version = PlatformVersion::latest();
        let dpns = get_dpns_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = dpns.data_contract_owned();
        let contract_info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(data_contract),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![],
        };
        ResolvedVote::ResolvedResourceVote(ResolvedResourceVote::V0(ResolvedResourceVoteV0 {
            resolved_vote_poll: ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                contract_info,
            ),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        }))
    }

    fn make_v0() -> MasternodeVoteTransitionActionV0 {
        MasternodeVoteTransitionActionV0 {
            pro_tx_hash: Identifier::from([0xAA; 32]),
            voter_identity_id: Identifier::from([0xBB; 32]),
            voting_address: [0xCC; 20],
            vote_strength: 4,
            vote: make_resolved_vote(),
            previous_resource_vote_choice_to_remove: Some((ResourceVoteChoice::Lock, 2)),
            nonce: 77,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: MasternodeVoteTransitionAction = v0.into();
        assert!(matches!(action, MasternodeVoteTransitionAction::V0(_)));
    }

    #[test]
    fn test_pro_tx_hash() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        assert_eq!(action.pro_tx_hash(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_voter_identity_id() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        assert_eq!(action.voter_identity_id(), Identifier::from([0xBB; 32]));
    }

    #[test]
    fn test_voting_address() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        assert_eq!(action.voting_address(), [0xCC; 20]);
    }

    #[test]
    fn test_vote_ref() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        let vote = action.vote_ref();
        assert!(matches!(vote, ResolvedVote::ResolvedResourceVote(_)));
    }

    #[test]
    fn test_vote_owned() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        let vote = action.vote_owned();
        assert!(matches!(vote, ResolvedVote::ResolvedResourceVote(_)));
    }

    #[test]
    fn test_nonce() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        assert_eq!(action.nonce(), 77);
    }

    #[test]
    fn test_vote_strength() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        assert_eq!(action.vote_strength(), 4);
    }

    #[test]
    fn test_previous_resource_vote_choice_to_remove() {
        let action = MasternodeVoteTransitionAction::V0(make_v0());
        let prev = action.previous_resource_vote_choice_to_remove();
        assert!(prev.is_some());
        let (choice, count) = prev.as_ref().unwrap();
        assert_eq!(*choice, ResourceVoteChoice::Lock);
        assert_eq!(*count, 2);
    }

    #[test]
    fn test_take_previous_resource_vote_choice_to_remove() {
        let mut action = MasternodeVoteTransitionAction::V0(make_v0());
        let taken = action.take_previous_resource_vote_choice_to_remove();
        assert!(taken.is_some());
        let (choice, count) = taken.unwrap();
        assert_eq!(choice, ResourceVoteChoice::Lock);
        assert_eq!(count, 2);
        // After taking, it should be None
        assert!(action.previous_resource_vote_choice_to_remove().is_none());
    }

    #[test]
    fn test_previous_resource_vote_choice_none() {
        let mut v0 = make_v0();
        v0.previous_resource_vote_choice_to_remove = None;
        let action = MasternodeVoteTransitionAction::V0(v0);
        assert!(action.previous_resource_vote_choice_to_remove().is_none());
    }
}
