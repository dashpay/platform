use crate::voting::Vote;
use platform_value::Identifier;

pub trait MasternodeVoteTransitionAccessorsV0 {
    fn pro_tx_hash(&self) -> Identifier;
    fn set_pro_tx_hash(&mut self, pro_tx_hash: Identifier);
    fn vote(&self) -> &Vote;
    fn set_vote(&mut self, vote: Vote);
}
