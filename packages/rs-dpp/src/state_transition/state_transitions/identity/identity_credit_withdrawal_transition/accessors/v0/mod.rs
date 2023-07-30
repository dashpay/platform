use crate::prelude::Revision;

pub trait IdentityCreditWithdrawalTransitionAccessorsV0 {
    fn set_revision(&mut self, revision: Revision);
    fn revision(&self) -> Revision;
}
