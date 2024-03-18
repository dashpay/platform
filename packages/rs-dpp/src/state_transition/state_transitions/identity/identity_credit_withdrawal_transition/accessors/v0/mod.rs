use crate::identity::core_script::CoreScript;
use crate::prelude::IdentityNonce;
use crate::withdrawal::Pooling;
use platform_value::Identifier;

pub trait IdentityCreditWithdrawalTransitionAccessorsV0 {
    fn identity_id(&self) -> Identifier;
    fn set_identity_id(&mut self, identity_id: Identifier);
    fn amount(&self) -> u64;
    fn set_amount(&mut self, amount: u64);
    fn nonce(&self) -> IdentityNonce;
    fn set_nonce(&mut self, nonce: IdentityNonce);
    fn pooling(&self) -> Pooling;
    fn set_pooling(&mut self, pooling: Pooling);
    fn core_fee_per_byte(&self) -> u32;
    fn set_core_fee_per_byte(&mut self, amount: u32);
    fn output_script(&self) -> CoreScript;
    fn set_output_script(&mut self, output_script: CoreScript);
}
