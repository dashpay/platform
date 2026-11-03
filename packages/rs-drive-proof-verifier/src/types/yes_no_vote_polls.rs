//! The state of a yes/no vote poll (`getYesNoVotePollState`) and a masternode's answer on one
//! (`getMasternodeYesNoVote`), both read by the poll's id.
//!
//! A poll's id is derived from the whole poll by `YesNoVotePoll::unique_id`, as the platform
//! version selects. Both results are Drive's own types, which its proof verifiers return.

pub use drive::query::masternode_yes_no_vote_query::MasternodeYesNoVote;
pub use drive::query::yes_no_vote_poll_state_query::YesNoVotePollState;
