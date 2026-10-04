use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_burn_from_pool_transition::v0::v0_methods::TokenBurnFromPoolTransitionV0Methods;
use crate::state_transition::batch_transition::TokenBurnFromPoolTransition;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

impl TokenBaseTransitionAccessors for TokenBurnFromPoolTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenBurnFromPoolTransitionV0Methods for TokenBurnFromPoolTransition {
    fn amount(&self) -> TokenAmount {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.amount(),
        }
    }
    fn actions(&self) -> &[SerializedAction] {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.actions(),
        }
    }
    fn anchor(&self) -> &[u8; 32] {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.anchor(),
        }
    }
    fn proof(&self) -> &[u8] {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.proof(),
        }
    }
    fn binding_signature(&self) -> &[u8; 64] {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.binding_signature(),
        }
    }
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.public_note(),
        }
    }
    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.public_note_owned(),
        }
    }
    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.set_amount(amount),
        }
    }
    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenBurnFromPoolTransition {
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenBurnFromPoolTransition::V0(v0) => {
                v0.calculate_action_id(owner_id, platform_version)
            }
        }
    }
}

impl TokenBurnFromPoolTransition {
    /// The group action id: unlike a transparent burn, the digest of the Orchard
    /// actions is part of it, so two proposals that differ only in the notes never share an id.
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        amount: TokenAmount,
        actions_digest: &[u8; 32],
    ) -> Identifier {
        let mut bytes = b"action_token_burn_from_pool".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&amount.to_be_bytes());
        bytes.extend_from_slice(actions_digest);

        hash_double(bytes).into()
    }
}
