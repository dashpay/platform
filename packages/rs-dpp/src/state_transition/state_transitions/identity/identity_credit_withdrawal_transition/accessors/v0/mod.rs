use crate::identity::core_script::CoreScript;
use crate::prelude::Revision;
use crate::withdrawal::Pooling;
use platform_value::Identifier;

pub trait IdentityCreditWithdrawalTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn amount(&self) -> u64;
    fn set_revision(&mut self, revision: Revision);
    fn revision(&self) -> Revision;
    fn pooling(&self) -> Pooling;
    fn core_fee_per_byte(&self) -> u32;
    fn output_script(&self) -> CoreScript;
}
