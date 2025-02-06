use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use crate::state_transition::batch_transition::batched_transition::token_release_transition::TokenReleaseTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenReleaseTransitionV0 {
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
pub trait TokenReleaseTransitionV0Methods {
    /// Returns the `recipient` field of the `TokenReleaseTransitionV0`.
    fn recipient(&self) -> &TokenDistributionRecipient;

    /// Returns the owned `recipient` field of the `TokenReleaseTransitionV0`.
    fn recipient_owned(self) -> TokenDistributionRecipient;

    /// Sets the `recipient` field in the `TokenReleaseTransitionV0`.
    fn set_recipient(&mut self, recipient: TokenDistributionRecipient);

    /// Returns the `distribution_type` field of the `TokenReleaseTransitionV0`.
    fn distribution_type(&self) -> TokenDistributionType;

    /// Returns the owned `distribution_type` field of the `TokenReleaseTransitionV0`.
    fn distribution_type_owned(self) -> TokenDistributionType;

    /// Sets the `distribution_type` field in the `TokenReleaseTransitionV0`.
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType);

    /// Returns the `public_note` field of the `TokenReleaseTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenReleaseTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenReleaseTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenReleaseTransitionV0Methods for TokenReleaseTransitionV0 {
    fn recipient(&self) -> &TokenDistributionRecipient {
        &self.recipient
    }

    fn recipient_owned(self) -> TokenDistributionRecipient {
        self.recipient
    }

    fn set_recipient(&mut self, recipient: TokenDistributionRecipient) {
        self.recipient = recipient;
    }

    fn distribution_type(&self) -> TokenDistributionType {
        self.distribution_type.clone()
    }

    fn distribution_type_owned(self) -> TokenDistributionType {
        self.distribution_type
    }

    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        self.distribution_type = distribution_type;
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }
}

// impl AllowedAsMultiPartyAction for TokenReleaseTransitionV0 {
//     fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
//         let TokenReleaseTransitionV0 {
//             base, recipient, distribution_type, ..
//
//         } = self;
//
//         TokenReleaseTransition::calculate_action_id_with_fields(
//             base.token_id().as_bytes(),
//             owner_id.as_bytes(),
//             base.identity_contract_nonce(),
//             recipient,
//             *distribution_type,
//         )
//     }
// }
