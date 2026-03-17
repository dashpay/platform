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
                    payload.as_deref(),
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
    fn v1_same_discriminant_different_values_produces_different_ids_through_production_path() {
        // This is the core regression test for the vote-swap fix.
        // Uses the full production calculate_action_id path on the latest
        // platform version (v1) to prove that different MaxSupply values
        // produce different action_ids.
        let owner_id = Identifier::new([3u8; 32]);
        let platform_version = PlatformVersion::latest();

        let t_small = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(100)));
        let t_large = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(
            999_999_999_999,
        )));

        let id_small = t_small
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id_large = t_large
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        // v1: these must be DIFFERENT -- the fix
        assert_ne!(
            id_small, id_large,
            "v1 should produce different action_ids for different MaxSupply values"
        );
    }

    #[test]
    fn v1_different_item_types_produces_different_ids_through_production_path() {
        let owner_id = Identifier::new([3u8; 32]);
        let platform_version = PlatformVersion::latest();

        let t_max = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(100)));
        let t_dest =
            make_transition(TokenConfigurationChangeItem::MintingAllowChoosingDestination(true));

        let id_max = t_max
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id_dest = t_dest
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        assert_ne!(
            id_max, id_dest,
            "v1 should produce different action_ids for different config item types"
        );
    }

    #[test]
    fn v0_and_v1_produce_different_ids_for_same_input() {
        // Verify v0 (first platform version) and v1 (latest) produce
        // different action_ids for the same config item.
        let owner_id = Identifier::new([3u8; 32]);

        let t = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(100)));

        let id_v0 = t
            .calculate_action_id(owner_id, PlatformVersion::first())
            .expect("expected action id");
        let id_v1 = t
            .calculate_action_id(owner_id, PlatformVersion::latest())
            .expect("expected action id");

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
            item.payload_serialization()
                .expect("expected to serialize payload")
                .as_deref(),
        );
        assert_eq!(
            id_versioned, id_v1,
            "versioned dispatch should use v1 on current platform version"
        );
    }

    #[test]
    fn v1_identical_items_produces_same_id_through_production_path() {
        // Sanity check: identical config items should produce the same
        // action_id under v1, through the production path.
        let owner_id = Identifier::new([3u8; 32]);
        let platform_version = PlatformVersion::latest();

        let t1 = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(42)));
        let t2 = make_transition(TokenConfigurationChangeItem::MaxSupply(Some(42)));

        let id1 = t1
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id2 = t2
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        assert_eq!(
            id1, id2,
            "v1 should produce the same action_id for identical config items"
        );
    }

    #[test]
    fn v1_authorized_action_takers_variant_differentiates_values() {
        // Exercises payload_serialization() for AuthorizedActionTakers-based
        // variants, which use to_bytes() internally. A regression in
        // AuthorizedActionTakers::to_bytes() or payload_serialization()
        // would be caught here.
        use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;

        let owner_id = Identifier::new([3u8; 32]);
        let platform_version = PlatformVersion::latest();

        let t_group5 = make_transition(TokenConfigurationChangeItem::ManualMinting(
            AuthorizedActionTakers::Group(5),
        ));
        let t_group9 = make_transition(TokenConfigurationChangeItem::ManualMinting(
            AuthorizedActionTakers::Group(9),
        ));
        let t_no_one = make_transition(TokenConfigurationChangeItem::ManualMinting(
            AuthorizedActionTakers::NoOne,
        ));

        let id_group5 = t_group5
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id_group9 = t_group9
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");
        let id_no_one = t_no_one
            .calculate_action_id(owner_id, platform_version)
            .expect("expected action id");

        assert_ne!(
            id_group5, id_group9,
            "ManualMinting(Group(5)) and ManualMinting(Group(9)) must differ"
        );
        assert_ne!(
            id_group5, id_no_one,
            "ManualMinting(Group(5)) and ManualMinting(NoOne) must differ"
        );
    }

    #[test]
    fn v1_none_payload_variants_still_differentiated_by_discriminant() {
        // MaxSupply(None), NewTokensDestinationIdentity(None),
        // PerpetualDistribution(None), and MainControlGroup(None) all
        // return None from payload_serialization(). They must still produce
        // different action_ids because of their different u8 discriminants.
        let owner_id = Identifier::new([3u8; 32]);
        let platform_version = PlatformVersion::latest();

        let t_max = make_transition(TokenConfigurationChangeItem::MaxSupply(None));
        let t_dest = make_transition(TokenConfigurationChangeItem::NewTokensDestinationIdentity(
            None,
        ));
        let t_dist = make_transition(TokenConfigurationChangeItem::PerpetualDistribution(None));
        let t_ctrl = make_transition(TokenConfigurationChangeItem::MainControlGroup(None));

        let ids: Vec<_> = [t_max, t_dest, t_dist, t_ctrl]
            .iter()
            .map(|t| {
                t.calculate_action_id(owner_id, platform_version)
                    .expect("expected action id")
            })
            .collect();

        // All 4 must be unique
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(
                    ids[i], ids[j],
                    "None-payload variants at indices {i} and {j} must have different action_ids"
                );
            }
        }
    }
}
