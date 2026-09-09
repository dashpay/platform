use bincode::{Decode, Encode};
use derive_more::From;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod document_base_transition;
pub mod document_create_transition;
pub mod document_delete_transition;
pub mod document_index_only_delete_transition;
pub mod document_purchase_transition;
pub mod document_replace_transition;
pub mod document_transfer_transition;
pub mod document_transition;
pub mod document_transition_action_type;
pub mod document_update_price_transition;
pub mod multi_party_action;
mod resolvers;
pub mod token_base_transition;
pub mod token_burn_transition;
pub mod token_claim_transition;
pub mod token_config_update_transition;
pub mod token_destroy_frozen_funds_transition;
pub mod token_direct_purchase_transition;
pub mod token_emergency_action_transition;
pub mod token_freeze_transition;
pub mod token_mint_transition;
pub mod token_set_price_for_direct_purchase_transition;
pub mod token_transfer_transition;
pub mod token_transition;
pub mod token_transition_action_type;
pub mod token_unfreeze_transition;

use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use derive_more::Display;
pub use document_create_transition::DocumentCreateTransition;
pub use document_delete_transition::DocumentDeleteTransition;
pub use document_index_only_delete_transition::DocumentIndexOnlyDeleteTransition;
pub use document_purchase_transition::DocumentPurchaseTransition;
pub use document_replace_transition::DocumentReplaceTransition;
pub use document_transfer_transition::DocumentTransferTransition;
pub use document_transition::DocumentTransition;
pub use document_update_price_transition::DocumentUpdatePriceTransition;
use platform_value::Identifier;
pub use token_transition::TokenTransition;

pub const PROPERTY_ACTION: &str = "$action";

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    // Internal tagging with the system-field key `$transition` — distinct
    // from the inner umbrellas' `$type` discriminator so both flatten into
    // the same wire shape without collision. Per the wasm-dpp2 convention
    // (no `data` wrapper), the inner umbrella's fields appear at the top
    // level alongside `$transition`. Resulting wire shape:
    //   { "$transition": "document", "$type": "create", "$formatVersion": "0", ... }
    serde(tag = "$transition", rename_all = "camelCase")
)]
pub enum BatchedTransition {
    #[display("DocumentTransition({})", "_0")]
    Document(DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(TokenTransition),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for BatchedTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for BatchedTransition {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::batched_transition::{
        document_create_transition, token_burn_transition,
    };
    use document_transition::DocumentTransition;
    use token_transition::TokenTransition;

    /// Internally tagged with `$transition` — wire shape is
    /// `{"$transition": "<variant>", "$type": "<inner>", ...inner fields}`.
    /// Both discriminators sit at the top level (no envelope nesting).
    fn assert_umbrella_round_trip(transition: BatchedTransition, expected_transition: &str) {
        use crate::serialization::{JsonConvertible, ValueConvertible};

        let json = transition.to_json().expect("to_json");
        let json_obj = json.as_object().expect("json object");
        assert_eq!(
            json_obj.get("$transition").and_then(|v| v.as_str()),
            Some(expected_transition),
            "json `$transition` discriminator mismatch"
        );
        assert!(
            json_obj.get("$action").and_then(|v| v.as_str()).is_some(),
            "json inner `$action` discriminator missing"
        );
        let recovered_json = BatchedTransition::from_json(json).expect("from_json");
        assert_eq!(transition, recovered_json);

        let value = transition.to_object().expect("to_object");
        let value_map = value.as_map().expect("value map");
        let kv = value_map
            .iter()
            .find(|(k, _)| matches!(k, platform_value::Value::Text(s) if s == "$transition"))
            .expect("$transition key present");
        assert_eq!(
            kv.1,
            platform_value::Value::Text(expected_transition.to_string()),
            "value `$transition` discriminator mismatch"
        );
        let recovered_value = BatchedTransition::from_object(value).expect("from_object");
        assert_eq!(transition, recovered_value);
    }

    #[test]
    fn umbrella_document() {
        let inner = DocumentTransition::Create(
            document_create_transition::json_convertible_tests::fixture(),
        );
        assert_umbrella_round_trip(BatchedTransition::Document(inner), "document");
    }

    #[test]
    fn umbrella_token() {
        let inner = TokenTransition::Burn(token_burn_transition::json_convertible_tests::fixture());
        assert_umbrella_round_trip(BatchedTransition::Token(inner), "token");
    }
}

#[derive(Debug, From, Clone, Copy, PartialEq, Display)]
pub enum BatchedTransitionRef<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(&'a DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(&'a TokenTransition),
}

#[derive(Debug, From, PartialEq, Display)]
pub enum BatchedTransitionMutRef<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(&'a mut DocumentTransition),
    #[display("TokenTransition({})", "_0")]
    Token(&'a mut TokenTransition),
}

impl BatchedTransitionRef<'_> {
    pub fn to_owned_transition(&self) -> BatchedTransition {
        match self {
            BatchedTransitionRef::Document(doc_ref) => {
                BatchedTransition::Document((*doc_ref).clone())
            }
            BatchedTransitionRef::Token(tok_ref) => BatchedTransition::Token((*tok_ref).clone()),
        }
    }

    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            BatchedTransitionRef::Document(document_transition) => {
                document_transition.identity_contract_nonce()
            }
            BatchedTransitionRef::Token(token_transition) => {
                token_transition.identity_contract_nonce()
            }
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        match self {
            BatchedTransitionRef::Document(document_transition) => {
                document_transition.data_contract_id()
            }
            BatchedTransitionRef::Token(token_transition) => token_transition.data_contract_id(),
        }
    }
}

impl BatchedTransition {
    pub fn borrow_as_ref(&self) -> BatchedTransitionRef<'_> {
        match self {
            BatchedTransition::Document(doc) => {
                // Create a reference to a DocumentTransition
                BatchedTransitionRef::Document(doc)
            }
            BatchedTransition::Token(tok) => {
                // Create a reference to a TokenTransition
                BatchedTransitionRef::Token(tok)
            }
        }
    }

    pub fn borrow_as_mut(&mut self) -> BatchedTransitionMutRef<'_> {
        match self {
            BatchedTransition::Document(doc) => {
                // Create a reference to a DocumentTransition
                BatchedTransitionMutRef::Document(doc)
            }
            BatchedTransition::Token(tok) => {
                // Create a reference to a TokenTransition
                BatchedTransitionMutRef::Token(tok)
            }
        }
    }

    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            BatchedTransition::Document(document_transition) => {
                document_transition.set_identity_contract_nonce(identity_contract_nonce)
            }
            BatchedTransition::Token(token_transition) => {
                token_transition.set_identity_contract_nonce(identity_contract_nonce)
            }
        }
    }
}
