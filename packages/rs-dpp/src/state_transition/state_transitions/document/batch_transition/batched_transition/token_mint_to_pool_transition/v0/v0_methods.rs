use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_mint_to_pool_transition::TokenMintToPoolTransition;
use crate::state_transition::batch_transition::token_mint_to_pool_transition::TokenMintToPoolTransitionV0;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::ProtocolError;

impl TokenBaseTransitionAccessors for TokenMintToPoolTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenMintToPoolTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Tokens entering or leaving the pool. Equals the absolute value of the bundle's value balance.
    fn amount(&self) -> TokenAmount;
    /// Orchard actions.
    fn actions(&self) -> &[SerializedAction];
    /// Sinsemilla root of the token pool's note commitment tree (Orchard anchor).
    fn anchor(&self) -> &[u8; 32];
    /// Halo 2 proof bytes.
    fn proof(&self) -> &[u8];
    /// RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
    /// Optional public note. Only a group action proposer may set one.
    fn public_note(&self) -> Option<&String>;
    /// Takes the public note.
    fn public_note_owned(self) -> Option<String>;
    /// Sets `amount`.
    fn set_amount(&mut self, amount: TokenAmount);
    /// Sets `public_note`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenMintToPoolTransitionV0Methods for TokenMintToPoolTransitionV0 {
    fn amount(&self) -> TokenAmount {
        self.amount
    }
    fn actions(&self) -> &[SerializedAction] {
        &self.actions
    }
    fn anchor(&self) -> &[u8; 32] {
        &self.anchor
    }
    fn proof(&self) -> &[u8] {
        &self.proof
    }
    fn binding_signature(&self) -> &[u8; 64] {
        &self.binding_signature
    }
    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }
    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }
    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }
    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }
}

impl AllowedAsMultiPartyAction for TokenMintToPoolTransitionV0 {
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        Ok(TokenMintToPoolTransition::calculate_action_id_with_fields(
            self.base.token_id().as_bytes(),
            owner_id.as_bytes(),
            self.base.identity_contract_nonce(),
            self.amount,
            &crate::shielded::serialized_actions_digest(&self.actions),
        ))
    }
}
