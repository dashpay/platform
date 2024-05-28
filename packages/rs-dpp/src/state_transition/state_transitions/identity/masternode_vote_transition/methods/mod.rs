mod v0;

use crate::identity::signer::Signer;
use crate::identity::IdentityPublicKey;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::{FeatureVersion, PlatformVersion};
pub use v0::*;

use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::StateTransition;
use crate::voting::votes::Vote;

impl MasternodeVoteTransitionMethodsV0 for MasternodeVoteTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_vote_with_signer<S: Signer>(
        vote: Vote,
        signer: &S,
        pro_tx_hash: Identifier,
        masternode_voting_key: &IdentityPublicKey,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match feature_version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .masternode_vote_state_transition
                .default_current_version,
        ) {
            0 => Ok(MasternodeVoteTransitionV0::try_from_vote_with_signer(
                vote,
                signer,
                pro_tx_hash,
                masternode_voting_key,
                nonce,
            )?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "MasternodeVoteTransition::try_from_vote_with_signer".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
