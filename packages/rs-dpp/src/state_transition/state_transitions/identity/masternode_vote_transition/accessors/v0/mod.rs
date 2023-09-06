use crate::voting::resource_vote::ResourceVote;
use platform_value::Identifier;

pub trait MasternodeVoteTransitionAccessorsV0 {
    fn pro_tx_hash(&self) -> Identifier;
    fn set_pro_tx_hash(&mut self, pro_tx_hash: Identifier);
    fn resource_vote(&self) -> ResourceVote;
    fn set_resource_vote(&mut self, resource_vote: ResourceVote);
}
