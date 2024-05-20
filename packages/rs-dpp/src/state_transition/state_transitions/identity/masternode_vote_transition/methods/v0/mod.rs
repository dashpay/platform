use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::IdentityNonce;
use crate::state_transition::{StateTransition, StateTransitionType};
use crate::voting::votes::Vote;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::{FeatureVersion, PlatformVersion};

pub trait MasternodeVoteTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_vote_with_signer<S: Signer>(
        vote: Vote,
        signer: &S,
        pro_tx_hash: Identifier,
        masternode_voting_key: &IdentityPublicKey,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::MasternodeVote
    }
}
