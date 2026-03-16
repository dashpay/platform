use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;
use crate::state_transition::batch_transition::TokenConfigUpdateTransition;

impl TokenBaseTransitionAccessors for TokenConfigUpdateTransitionV0 {
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

pub trait TokenConfigUpdateTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    /// Returns the `update_token_configuration_item`
    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem;

    /// Sets the `update_token_configuration_item`
    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    );

    /// Returns the `public_note` field of the `TokenConfigUpdateTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenConfigUpdateTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the `public_note` field in the `TokenConfigUpdateTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenConfigUpdateTransitionV0Methods for TokenConfigUpdateTransitionV0 {
    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        &self.update_token_configuration_item
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        self.update_token_configuration_item = update_token_configuration_item;
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

impl AllowedAsMultiPartyAction for TokenConfigUpdateTransitionV0 {
    /// v0 action_id: uses only the u8 discriminant (kept for backward compat).
    fn calculate_action_id(&self, owner_id: Identifier, platform_version: &PlatformVersion,) -> Result<Identifier, ProtocolError> {
        let TokenConfigUpdateTransitionV0 {
            base,
            update_token_configuration_item,
            ..
        } = self;

        match platform_version
            .dpp
            .token_versions
            .token_config_update_action_id_version
        {
            0 => Ok(TokenConfigUpdateTransition::calculate_action_id_with_fields_v0(
                base.token_id().as_bytes(),
                owner_id.as_bytes(),
                base.identity_contract_nonce(),
                update_token_configuration_item.u8_item_index(),
            )),
            1 => Ok(TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
                base.token_id().as_bytes(),
                owner_id.as_bytes(),
                base.identity_contract_nonce(),
                update_token_configuration_item,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "calculate_action_id".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
