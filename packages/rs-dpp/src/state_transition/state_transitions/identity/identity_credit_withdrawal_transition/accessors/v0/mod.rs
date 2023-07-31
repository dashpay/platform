use crate::prelude::Revision;
use platform_value::Identifier;

pub trait IdentityCreditWithdrawalTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn amount(&self) -> u64;
    fn set_revision(&mut self, revision: Revision);
    fn revision(&self) -> Revision;
}
