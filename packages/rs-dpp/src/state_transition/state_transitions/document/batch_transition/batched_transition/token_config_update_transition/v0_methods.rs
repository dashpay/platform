use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransition;
use crate::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenConfigUpdateTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenConfigUpdateTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenConfigUpdateTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenConfigUpdateTransitionV0Methods for TokenConfigUpdateTransition {
    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.update_token_configuration_item(),
        }
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => {
                v0.set_update_token_configuration_item(update_token_configuration_item)
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenConfigUpdateTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenConfigUpdateTransition {
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenConfigUpdateTransition::V0(v0) => {
                v0.calculate_action_id(owner_id, platform_version)
            }
        }
    }
}

impl TokenConfigUpdateTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        token_configuration_change_item: &TokenConfigurationChangeItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .token_config_update_action_id_version
        {
            0 => Ok(Self::calculate_action_id_with_fields_v0(
                token_id,
                owner_id,
                identity_contract_nonce,
                token_configuration_change_item.u8_item_index(),
            )),
            1 => {
                let payload = token_configuration_change_item.payload_serialization()?;
                Ok(Self::calculate_action_id_with_fields_v1(
                    token_id,
                    owner_id,
                    identity_contract_nonce,
                    token_configuration_change_item.u8_item_index(),
                    payload.as_ref().map(|a| a.as_slice()),
                ))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "calculate_action_id_with_fields".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
    /// v0: action_id uses only the u8 discriminant of the config change item.
    /// This is kept for backward compatibility with existing production data.
    fn calculate_action_id_with_fields_v0(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        update_token_config_item: u8,
    ) -> Identifier {
        let mut bytes = b"action_token_config_update".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&[update_token_config_item]);

        hash_double(bytes).into()
    }

    /// v1: action_id includes the u8 discriminant plus an optional serialized
    /// payload, binding the voted-on value into the hash and preventing
    /// vote-swap attacks.
    fn calculate_action_id_with_fields_v1(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        update_token_config_item: u8,
        payload: Option<&[u8]>,
    ) -> Identifier {
        let mut bytes = b"action_token_config_update".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.push(update_token_config_item);
        if let Some(payload) = payload {
            bytes.extend_from_slice(payload);
        }

        hash_double(bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_config_update_transition::TokenConfigUpdateTransitionV0;

    fn serialize_item(item: &TokenConfigurationChangeItem) -> Vec<u8> {
        bincode::encode_to_vec(item, bincode::config::standard()).expect("expected to encode item")
    }

    fn make_transition(item: TokenConfigurationChangeItem) -> TokenConfigUpdateTransition {
        TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::new([1u8; 32]),
                token_id: Identifier::new([2u8; 32]),
                using_group_info: None,
            }),
            update_token_configuration_item: item,
            public_note: None,
        })
    }

    #[test]
    fn v0_action_id_same_discriminant_different_values_produces_same_id_vulnerability() {
        // This test documents the v0 vulnerability: two different MaxSupply
        // values produce the same action_id because only the u8 discriminant
        // is hashed, not the actual value.
        let owner_id = Identifier::new([3u8; 32]);

        let t_small = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(100)));
        let t_large = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(
            999_999_999_999,
        )));

        let platform_version = PlatformVersion::first();
        let id_small = t_small
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id_large = t_large
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        // v0: these are EQUAL -- the vulnerability
        assert_eq!(
            id_small, id_large,
            "v0 should produce the same action_id for different MaxSupply values (vulnerability)"
        );
    }

    #[test]
    fn v1_action_id_same_discriminant_different_values_produces_different_ids() {
        // After the fix, the full serialized config item is included in the
        // hash, so different values produce different action_ids.
        let token_id = [2u8; 32];
        let owner_id = [3u8; 32];
        let nonce = 1u64;

        let item_small = TokenConfigurationChangeItem::MaxSupply(Some(100));
        let item_large = TokenConfigurationChangeItem::MaxSupply(Some(999_999_999_999));

        let id_small = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item_small.u8_item_index(),
            Some(&serialize_item(&item_small)),
        );
        let id_large = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item_large.u8_item_index(),
            Some(&serialize_item(&item_large)),
        );

        // v1: these must be DIFFERENT -- the fix
        assert_ne!(
            id_small, id_large,
            "v1 should produce different action_ids for different MaxSupply values"
        );
    }

    #[test]
    fn v1_action_id_different_item_types_produces_different_ids() {
        let token_id = [2u8; 32];
        let owner_id = [3u8; 32];
        let nonce = 1u64;

        let item_max_supply = TokenConfigurationChangeItem::MaxSupply(Some(100));
        let item_allow_dest = TokenConfigurationChangeItem::MintingAllowChoosingDestination(true);

        let id_max = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item_max_supply.u8_item_index(),
            Some(&serialize_item(&item_max_supply)),
        );
        let id_dest = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item_allow_dest.u8_item_index(),
            Some(&serialize_item(&item_allow_dest)),
        );

        assert_ne!(
            id_max, id_dest,
            "v1 should produce different action_ids for different config item types"
        );
    }

    #[test]
    fn v0_and_v1_produce_different_ids_for_same_input() {
        // Verify v0 and v1 are not accidentally identical (they hash
        // different payloads).
        let token_id = [2u8; 32];
        let owner_id = [3u8; 32];
        let nonce = 1u64;
        let item = TokenConfigurationChangeItem::MaxSupply(Some(100));

        let id_v0 = TokenConfigUpdateTransition::calculate_action_id_with_fields_v0(
            &token_id,
            &owner_id,
            nonce,
            item.u8_item_index(),
        );
        let id_v1 = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item.u8_item_index(),
            Some(&serialize_item(&item)),
        );

        assert_ne!(
            id_v0, id_v1,
            "v0 and v1 should produce different action_ids for the same config item"
        );
    }

    #[test]
    fn versioned_dispatch_uses_v1_on_current_platform_version() {
        // On the current platform version (v12), token_config_update_action_id_version
        // is 1, so the versioned method should produce the v1 result (which includes
        // the full config item payload), NOT the v0 result (discriminant only).
        let owner_id = Identifier::new([3u8; 32]);
        let t = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(100)));

        let platform_version = PlatformVersion::latest();

        let id_plain_v0 = t
            .calculate_action_id(owner_id, PlatformVersion::first())
            .expect("expected action id");
        let id_versioned = t
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        // v1 produces a different id from v0 because it hashes the full payload
        assert_ne!(
            id_plain_v0, id_versioned,
            "on current platform version (v1), versioned should differ from plain (v0)"
        );

        // Verify the versioned result matches v1 directly
        let base = t.base();
        let item = t.update_token_configuration_item();
        let id_v1 = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            item.u8_item_index(),
            Some(&serialize_item(item)),
        );
        assert_eq!(
            id_versioned, id_v1,
            "versioned dispatch should use v1 on current platform version"
        );
    }

    #[test]
    fn v1_action_id_identical_items_produces_same_id() {
        // Sanity check: identical config items should produce the same
        // action_id under v1.
        let token_id = [2u8; 32];
        let owner_id = [3u8; 32];
        let nonce = 1u64;

        let item1 = TokenConfigurationChangeItem::MaxSupply(Some(42));
        let item2 = TokenConfigurationChangeItem::MaxSupply(Some(42));

        let id1 = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item1.u8_item_index(),
            Some(&serialize_item(&item1)),
        );
        let id2 = TokenConfigUpdateTransition::calculate_action_id_with_fields_v1(
            &token_id,
            &owner_id,
            nonce,
            item2.u8_item_index(),
            Some(&serialize_item(&item2)),
        );

        assert_eq!(
            id1, id2,
            "v1 should produce the same action_id for identical config items"
        );
    }
}
