use crate::prelude::IdentityNonce;
use crate::voting::votes::Vote;
use platform_value::Identifier;

pub trait MasternodeVoteTransitionAccessorsV0 {
    fn pro_tx_hash(&self) -> Identifier;
    fn voter_identity_id(&self) -> Identifier;
    fn set_pro_tx_hash(&mut self, pro_tx_hash: Identifier);
    fn set_voter_identity_id(&mut self, voter_id: Identifier);
    fn vote(&self) -> &Vote;
    fn set_vote(&mut self, vote: Vote);
    fn nonce(&self) -> IdentityNonce;
}
