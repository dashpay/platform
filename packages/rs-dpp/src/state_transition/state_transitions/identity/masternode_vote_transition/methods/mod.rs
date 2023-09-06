mod v0;

pub use v0::*;

use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;

impl MasternodeVoteTransitionMethodsV0 for MasternodeVoteTransition {}
