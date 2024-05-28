use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, SecurityLevel};
use crate::prelude::IdentityNonce;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::StateTransition;
use crate::voting::votes::Vote;
use crate::ProtocolError;
use platform_value::Identifier;

impl MasternodeVoteTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    pub fn try_from_vote_with_signer<S: Signer>(
        vote: Vote,
        signer: &S,
        pro_tx_hash: Identifier,
        masternode_voting_key: &IdentityPublicKey,
        nonce: IdentityNonce,
    ) -> Result<StateTransition, ProtocolError> {
        let masternode_vote_transition: MasternodeVoteTransition = MasternodeVoteTransitionV0 {
            pro_tx_hash,
            vote,
            nonce,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        let mut state_transition: StateTransition = masternode_vote_transition.into();
        state_transition.sign_external(
            masternode_voting_key,
            signer,
            None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
        )?;
        Ok(state_transition)
    }
}
