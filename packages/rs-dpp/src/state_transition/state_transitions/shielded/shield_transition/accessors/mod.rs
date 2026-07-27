mod v0;

pub use v0::*;

use crate::address_funds::AddressFundsFeeStrategy;
use crate::shielded::SerializedAction;
use crate::state_transition::shield_transition::ShieldTransition;

impl ShieldTransitionAccessorsV0 for ShieldTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            ShieldTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            ShieldTransition::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            ShieldTransition::V0(v0) => v0.amount = amount,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            ShieldTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            ShieldTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            ShieldTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            ShieldTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            ShieldTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            ShieldTransition::V0(v0) => v0.binding_signature = binding_signature,
        }
    }

    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            ShieldTransition::V0(v0) => &v0.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            ShieldTransition::V0(v0) => v0.fee_strategy = fee_strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
    use std::collections::BTreeMap;

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

    fn make_transition() -> ShieldTransition {
        ShieldTransitionV0 {
            inputs: BTreeMap::new(),
            actions: vec![mk_action(0x11)],
            amount: 1_000,
            anchor: [7u8; 32],
            proof: vec![8u8; 10],
            binding_signature: [9u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        }
        .into()
    }

    #[test]
    fn test_getters_and_setters() {
        let mut t = make_transition();

        assert_eq!(t.actions(), &[mk_action(0x11)]);
        assert_eq!(t.amount(), 1_000);
        assert_eq!(t.anchor(), [7u8; 32]);
        assert_eq!(t.proof(), &[8u8; 10]);
        assert_eq!(t.binding_signature(), [9u8; 64]);
        assert!(t.fee_strategy().is_empty());

        t.set_actions(vec![mk_action(0x22), mk_action(0x33)]);
        assert_eq!(t.actions(), &[mk_action(0x22), mk_action(0x33)]);

        t.set_amount(42);
        assert_eq!(t.amount(), 42);

        t.set_anchor([1u8; 32]);
        assert_eq!(t.anchor(), [1u8; 32]);

        t.set_proof(vec![0xAu8; 5]);
        assert_eq!(t.proof(), &[0xAu8; 5]);

        t.set_binding_signature([0xBu8; 64]);
        assert_eq!(t.binding_signature(), [0xBu8; 64]);

        t.set_fee_strategy(vec![AddressFundsFeeStrategyStep::DeductFromInput(1)]);
        assert_eq!(
            t.fee_strategy(),
            &[AddressFundsFeeStrategyStep::DeductFromInput(1)]
        );
    }
}
