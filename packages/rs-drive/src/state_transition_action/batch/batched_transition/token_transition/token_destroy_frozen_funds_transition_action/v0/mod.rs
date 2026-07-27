mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenDestroyFrozenFundsTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The identity to credit the token to
    pub frozen_identity_id: Identifier,
    /// The amount that will be burned
    pub amount: TokenAmount,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenDestroyFrozenFundsTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Consumes self and returns the identity balance holder ID
    fn frozen_identity_id(&self) -> Identifier;

    /// Sets the identity balance holder ID
    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier);

    /// Returns the amount of tokens that the identity had and will be burned
    fn amount(&self) -> TokenAmount;

    /// Sets the amount of tokens that the identity had and will be burned
    fn set_amount(&mut self, amount: TokenAmount);

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenDestroyFrozenFundsTransitionActionAccessorsV0
    for TokenDestroyFrozenFundsTransitionActionV0
{
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn frozen_identity_id(&self) -> Identifier {
        self.frozen_identity_id
    }

    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier) {
        self.frozen_identity_id = frozen_identity_id;
    }

    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0xFE; 32]),
            identity_contract_nonce: 6,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(
        frozen: Identifier,
        amount: TokenAmount,
        note: Option<&str>,
    ) -> TokenDestroyFrozenFundsTransitionActionV0 {
        TokenDestroyFrozenFundsTransitionActionV0 {
            base: make_base(),
            frozen_identity_id: frozen,
            amount,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn v0_frozen_identity_id_returns_stored_value() {
        let id = Identifier::new([0x11; 32]);
        let v0 = make_v0(id, 0, None);
        assert_eq!(v0.frozen_identity_id(), id);
    }

    #[test]
    fn v0_set_frozen_identity_id_updates() {
        let mut v0 = make_v0(Identifier::new([0x11; 32]), 0, None);
        let newer = Identifier::new([0x22; 32]);
        v0.set_frozen_identity_id(newer);
        assert_eq!(v0.frozen_identity_id(), newer);
    }

    #[test]
    fn v0_amount_returns_stored_value() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 42, None);
        assert_eq!(v0.amount(), 42);
    }

    #[test]
    fn v0_set_amount_updates() {
        let mut v0 = make_v0(Identifier::new([0x11; 32]), 42, None);
        v0.set_amount(100);
        assert_eq!(v0.amount(), 100);
        // zero
        v0.set_amount(0);
        assert_eq!(v0.amount(), 0);
        // max
        v0.set_amount(u64::MAX);
        assert_eq!(v0.amount(), u64::MAX);
    }

    #[test]
    fn v0_public_note_returns_ref_when_set() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 1, Some("burning"));
        assert_eq!(v0.public_note(), Some(&"burning".to_string()));
    }

    #[test]
    fn v0_public_note_returns_none_when_unset() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 1, None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_public_note_owned_consumes_self() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 1, Some("owned-note"));
        let owned = v0.public_note_owned();
        assert_eq!(owned, Some("owned-note".to_string()));
    }

    #[test]
    fn v0_set_public_note_replaces_and_clears() {
        let mut v0 = make_v0(Identifier::new([0x11; 32]), 1, Some("orig"));
        v0.set_public_note(Some("swapped".to_string()));
        assert_eq!(v0.public_note(), Some(&"swapped".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_base_ref_and_base_owned_preserve_token_id() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 1, None);
        let ref_id = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(ref_id, base.token_id());
        assert_eq!(ref_id, Identifier::new([0xFE; 32]));
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0(Identifier::new([0x11; 32]), 1, None);
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0xFE; 32]));
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }
}
