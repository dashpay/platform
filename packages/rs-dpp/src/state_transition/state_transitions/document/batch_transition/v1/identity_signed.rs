use crate::identity::SecurityLevel::{CRITICAL, HIGH, MEDIUM};
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::token_transition_action_type::{
    TokenTransitionActionType, TokenTransitionActionTypeGetter,
};
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use crate::state_transition::batch_transition::BatchTransitionV1;
use crate::state_transition::StateTransitionIdentitySigned;

impl BatchTransitionV1 {
    /// The purposes a key may have to sign a version 1 batch. `lone_token_action` is the action
    /// of the batch's only transition when that is a token transition, and `None` for any other
    /// batch: several transitions, a document transition, or none. A TRANSFER key may sign a
    /// lone token transfer or claim; every other batch takes AUTHENTICATION only.
    ///
    /// [`StateTransitionIdentitySigned::purpose_requirement`] answers through this, so a client
    /// choosing a key before it builds the batch applies the rule consensus does.
    pub fn signing_key_purposes(
        lone_token_action: Option<TokenTransitionActionType>,
    ) -> &'static [Purpose] {
        use TokenTransitionActionType::*;
        match lone_token_action {
            Some(Transfer | Claim) => &[Purpose::AUTHENTICATION, Purpose::TRANSFER],
            Some(
                Burn
                | Mint
                | Freeze
                | Unfreeze
                | DestroyFrozenFunds
                | EmergencyAction
                | ConfigUpdate
                | DirectPurchase
                | SetPriceForDirectPurchase
                | Shield
                | MintToPool
                | BurnFromPool
                | ClaimToPool
                | DirectPurchaseToPool
                | Unshield
                | ShieldedTransfer,
            )
            | None => &[Purpose::AUTHENTICATION],
        }
    }
}

impl StateTransitionIdentitySigned for BatchTransitionV1 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        if purpose == Purpose::TRANSFER {
            vec![CRITICAL]
        } else {
            // These are the available key levels that must sign the state transition
            // However the fact that it is signed by one of these does not guarantee that it
            // meets the security level requirement, as that is dictated from within the data
            // contract
            vec![CRITICAL, HIGH, MEDIUM]
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        // Edited in place in a shipped generation (BatchTransitionV1, protocol versions 9 to
        // 15) as an output-identical refactor: `action_type()` is `Transfer` exactly for
        // `TokenTransition::Transfer` and `Claim` exactly for `TokenTransition::Claim`, the two
        // transitions the previous `as_transition_token_transfer` / `as_transition_token_claim`
        // checks matched, and every other batch still gets AUTHENTICATION alone.
        let lone_token_action = match (self.transitions_len(), self.first_transition()) {
            (1, Some(BatchedTransitionRef::Token(token))) => Some(token.action_type()),
            _ => None,
        };
        Self::signing_key_purposes(lone_token_action).to_vec()
    }
}
