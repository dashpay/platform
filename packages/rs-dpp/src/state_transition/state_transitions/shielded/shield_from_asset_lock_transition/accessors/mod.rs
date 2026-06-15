mod v0;

pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

impl ShieldFromAssetLockTransitionAccessorsV0 for ShieldFromAssetLockTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn value_balance(&self) -> u64 {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance,
        }
    }

    fn set_value_balance(&mut self, value_balance: u64) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance = value_balance,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.binding_signature = binding_signature,
        }
    }

    fn surplus_output(&self) -> Option<&PlatformAddress> {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.surplus_output.as_ref(),
        }
    }

    fn set_surplus_output(&mut self, surplus_output: Option<PlatformAddress>) {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.surplus_output = surplus_output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use dashcore::OutPoint;
    use platform_value::BinaryData;

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

    fn make_transition() -> ShieldFromAssetLockTransition {
        ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::from([11u8; 36]),
            }),
            actions: vec![mk_action(0x11)],
            value_balance: 1_000,
            anchor: [7u8; 32],
            proof: vec![8u8; 10],
            binding_signature: [9u8; 64],
            surplus_output: None,
            signature: BinaryData::new(vec![10u8; 65]),
        }
        .into()
    }

    #[test]
    fn test_getters_and_setters() {
        let mut t = make_transition();

        assert_eq!(t.actions(), &[mk_action(0x11)]);
        assert_eq!(t.value_balance(), 1_000);
        assert_eq!(t.anchor(), [7u8; 32]);
        assert_eq!(t.proof(), &[8u8; 10]);
        assert_eq!(t.binding_signature(), [9u8; 64]);
        assert_eq!(t.surplus_output(), None);

        t.set_actions(vec![mk_action(0x22)]);
        assert_eq!(t.actions(), &[mk_action(0x22)]);

        t.set_value_balance(42);
        assert_eq!(t.value_balance(), 42);

        t.set_anchor([1u8; 32]);
        assert_eq!(t.anchor(), [1u8; 32]);

        t.set_proof(vec![0xAu8; 5]);
        assert_eq!(t.proof(), &[0xAu8; 5]);

        t.set_binding_signature([0xBu8; 64]);
        assert_eq!(t.binding_signature(), [0xBu8; 64]);

        let surplus = PlatformAddress::P2pkh([3u8; 20]);
        t.set_surplus_output(Some(surplus));
        assert_eq!(t.surplus_output(), Some(&surplus));
    }
}
