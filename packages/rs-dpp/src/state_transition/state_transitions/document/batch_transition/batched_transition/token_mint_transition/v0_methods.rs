use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_mint_transition::{TokenMintTransition, TokenMintTransitionV1};
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenMintTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenMintTransition::V0(v0) => &v0.base,
            TokenMintTransition::V1(v1) => &v1.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenMintTransition::V0(v0) => &mut v0.base,
            TokenMintTransition::V1(v1) => &mut v1.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenMintTransition::V0(v0) => v0.base = base,
            TokenMintTransition::V1(v1) => v1.base = base,
        }
    }
}

impl TokenMintTransition {
    /// Sets the least the mint issues when its amount is random, taking the format that holds
    /// it: `Some` turns a version 0 mint into a version 1 one, and `None` turns a version 1 mint
    /// back into version 0, the format every protocol version accepts. Only the versioned mint
    /// has the setter: a bare `TokenMintTransitionV0` holds no random amount, so a setter on it
    /// could only drop the value.
    pub fn set_random_minimum_amount(&mut self, random_minimum_amount: Option<u64>) {
        let current = std::mem::take(self);
        *self = match (current, random_minimum_amount) {
            (TokenMintTransition::V0(v0), None) => TokenMintTransition::V0(v0),
            (TokenMintTransition::V0(v0), Some(minimum)) => {
                TokenMintTransition::V1(TokenMintTransitionV1::from_v0(v0, Some(minimum)))
            }
            (TokenMintTransition::V1(mut v1), Some(minimum)) => {
                v1.random_minimum_amount = Some(minimum);
                TokenMintTransition::V1(v1)
            }
            (TokenMintTransition::V1(v1), None) => {
                TokenMintTransition::V0(v1.into_v0_and_random_minimum_amount().0)
            }
        };
    }
}

impl TokenMintTransitionV0Methods for TokenMintTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenMintTransition::V0(v0) => v0.amount(),
            TokenMintTransition::V1(v1) => v1.amount(),
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_amount(amount),
            TokenMintTransition::V1(v1) => v1.set_amount(amount),
        }
    }

    fn random_minimum_amount(&self) -> Option<u64> {
        match self {
            TokenMintTransition::V0(_) => None,
            TokenMintTransition::V1(v1) => v1.random_minimum_amount,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenMintTransition::V0(v0) => v0.public_note(),
            TokenMintTransition::V1(v1) => v1.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenMintTransition::V0(v0) => v0.public_note_owned(),
            TokenMintTransition::V1(v1) => v1.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_public_note(public_note),
            TokenMintTransition::V1(v1) => v1.set_public_note(public_note),
        }
    }

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        match self {
            TokenMintTransition::V0(v0) => v0.issued_to_identity_id(),
            TokenMintTransition::V1(v1) => v1.issued_to_identity_id(),
        }
    }

    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenMintTransition::V0(v0) => v0.recipient_id(token_configuration),
            TokenMintTransition::V1(v1) => v1.recipient_id(token_configuration),
        }
    }

    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_issued_to_identity_id(issued_to_identity_id),
            TokenMintTransition::V1(v1) => v1.set_issued_to_identity_id(issued_to_identity_id),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenMintTransition {
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenMintTransition::V0(v0) => v0.calculate_action_id(owner_id, platform_version),
            TokenMintTransition::V1(v1) => v1.calculate_action_id(owner_id, platform_version),
        }
    }
}

impl TokenMintTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        mint_amount: TokenAmount,
    ) -> Identifier {
        let mut bytes = b"action_token_mint".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&mint_amount.to_be_bytes());

        hash_double(bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;

    fn mint_v0() -> TokenMintTransition {
        TokenMintTransition::V0(TokenMintTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 4,
                token_contract_position: 1,
                data_contract_id: Identifier::new([1; 32]),
                token_id: Identifier::new([2; 32]),
                using_group_info: None,
            }),
            issued_to_identity_id: Some(Identifier::new([3; 32])),
            amount: 500,
            public_note: Some("note".to_string()),
        })
    }

    #[test]
    fn should_take_the_format_that_holds_the_random_minimum_amount() {
        let original = mint_v0();
        let mut mint = original.clone();
        assert_eq!(mint.random_minimum_amount(), None);

        mint.set_random_minimum_amount(None);
        assert_eq!(mint, original, "a version 0 mint stays as it is");

        mint.set_random_minimum_amount(Some(10));
        assert!(matches!(mint, TokenMintTransition::V1(_)));
        assert_eq!(mint.random_minimum_amount(), Some(10));
        assert_eq!(mint.amount(), 500);
        assert_eq!(
            mint.issued_to_identity_id(),
            original.issued_to_identity_id()
        );
        assert_eq!(mint.public_note(), original.public_note());
        assert_eq!(mint.base(), original.base());
        assert_eq!(
            mint.calculate_action_id(Identifier::new([9; 32]), PlatformVersion::latest())
                .expect("an action id"),
            original
                .calculate_action_id(Identifier::new([9; 32]), PlatformVersion::latest())
                .expect("an action id"),
            "the random minimum amount takes no part in the action id"
        );

        mint.set_random_minimum_amount(None);
        assert_eq!(
            mint, original,
            "without a random amount the mint is version 0 again"
        );
    }
}
