use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_release_transition::v0::v0_methods::TokenReleaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenReleaseTransition;

impl TokenBaseTransitionAccessors for TokenReleaseTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenReleaseTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenReleaseTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenReleaseTransition::V0(v0) => v0.base = base,
        }
    }
}
impl TokenReleaseTransitionV0Methods for TokenReleaseTransition {
    fn recipient(&self) -> &TokenDistributionRecipient {
        match self {
            TokenReleaseTransition::V0(v0) => v0.recipient(),
        }
    }

    fn recipient_owned(self) -> TokenDistributionRecipient {
        match self {
            TokenReleaseTransition::V0(v0) => v0.recipient_owned(),
        }
    }

    fn set_recipient(&mut self, recipient: TokenDistributionRecipient) {
        match self {
            TokenReleaseTransition::V0(v0) => v0.set_recipient(recipient),
        }
    }

    fn distribution_type(&self) -> TokenDistributionType {
        match self {
            TokenReleaseTransition::V0(v0) => v0.distribution_type(),
        }
    }

    fn distribution_type_owned(self) -> TokenDistributionType {
        match self {
            TokenReleaseTransition::V0(v0) => v0.distribution_type_owned(),
        }
    }

    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        match self {
            TokenReleaseTransition::V0(v0) => v0.set_distribution_type(distribution_type),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenReleaseTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenReleaseTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenReleaseTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

// impl AllowedAsMultiPartyAction for TokenReleaseTransition {
//     fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
//         match self {
//             TokenReleaseTransition::V0(v0) => v0.calculate_action_id(owner_id),
//         }
//     }
// }

// impl TokenReleaseTransition {
//     pub fn calculate_action_id_with_fields(
//         token_id: &[u8; 32],
//         owner_id: &[u8; 32],
//         identity_contract_nonce: IdentityNonce,
//         recipient: &TokenDistributionRecipient,
//         token_distribution_type: TokenDistributionType,
//     ) -> Identifier {
//         let mut bytes = b"action_token_release".to_vec();
//         bytes.extend_from_slice(token_id);
//         bytes.extend_from_slice(owner_id);
//         bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
//         bytes.extend_from_slice(recipient.serialize_to_bytes().expect("expected to be able to serialize").as_slice());
//         bytes.push(token_distribution_type as u8);
//
//         hash_double(bytes).into()
//     }
// }
