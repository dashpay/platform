use crate::prelude::Revision;
use platform_value::Identifier;
use crate::identity::core_script::CoreScript;
use crate::withdrawal::Pooling;

pub trait IdentityCreditWithdrawalTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn amount(&self) -> u64;
    fn set_revision(&mut self, revision: Revision);
    fn revision(&self) -> Revision;
    fn pooling(&self) -> Pooling;
    fn core_fee_per_byte(&self) -> u32;
    fn output_script(&self) -> CoreScript;
}

