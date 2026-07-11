mod v0;

pub use v0::*;

use crate::identity::core_script::CoreScript;
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::withdrawal::Pooling;

impl ShieldedWithdrawalTransitionAccessorsV0 for ShieldedWithdrawalTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn unshielding_amount(&self) -> u64 {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount,
        }
    }

    fn set_unshielding_amount(&mut self, unshielding_amount: u64) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.unshielding_amount = unshielding_amount,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.binding_signature = binding_signature,
        }
    }

    fn core_fee_per_byte(&self) -> u32 {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.core_fee_per_byte = core_fee_per_byte,
        }
    }

    fn pooling(&self) -> Pooling {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.pooling,
        }
    }

    fn set_pooling(&mut self, pooling: Pooling) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.pooling = pooling,
        }
    }

    fn output_script(&self) -> &CoreScript {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => &v0.output_script,
        }
    }

    fn set_output_script(&mut self, output_script: CoreScript) {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.output_script = output_script,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;

    fn mk_action(nullifier_byte: u8) -> SerializedAction {
        SerializedAction {
            nullifier: [nullifier_byte; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn make_transition() -> ShieldedWithdrawalTransition {
        ShieldedWithdrawalTransitionV0 {
            actions: vec![mk_action(0x11)],
            unshielding_amount: 1_000,
            anchor: [7u8; 32],
            proof: vec![8u8; 10],
            binding_signature: [9u8; 64],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([11u8; 20]),
        }
        .into()
    }

    #[test]
    fn test_getters_and_setters() {
        let mut t = make_transition();

        assert_eq!(t.actions(), &[mk_action(0x11)]);
        assert_eq!(t.unshielding_amount(), 1_000);
        assert_eq!(t.anchor(), [7u8; 32]);
        assert_eq!(t.proof(), &[8u8; 10]);
        assert_eq!(t.binding_signature(), [9u8; 64]);
        assert_eq!(t.core_fee_per_byte(), 1);
        assert_eq!(t.pooling(), Pooling::Never);
        assert_eq!(t.output_script(), &CoreScript::new_p2pkh([11u8; 20]));

        t.set_actions(vec![mk_action(0x22)]);
        assert_eq!(t.actions(), &[mk_action(0x22)]);

        t.set_unshielding_amount(42);
        assert_eq!(t.unshielding_amount(), 42);

        t.set_anchor([1u8; 32]);
        assert_eq!(t.anchor(), [1u8; 32]);

        t.set_proof(vec![0xAu8; 5]);
        assert_eq!(t.proof(), &[0xAu8; 5]);

        t.set_binding_signature([0xBu8; 64]);
        assert_eq!(t.binding_signature(), [0xBu8; 64]);

        t.set_core_fee_per_byte(7);
        assert_eq!(t.core_fee_per_byte(), 7);

        t.set_pooling(Pooling::Standard);
        assert_eq!(t.pooling(), Pooling::Standard);

        t.set_output_script(CoreScript::new_p2pkh([22u8; 20]));
        assert_eq!(t.output_script(), &CoreScript::new_p2pkh([22u8; 20]));
    }
}
