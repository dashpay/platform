use crate::prelude::UserFeeIncrease;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::state_transition::StateTransitionType::Batch;
use crate::state_transition::{StateTransition, StateTransitionLike, StateTransitionOwned, StateTransitionSingleSigned, StateTransitionType};
use crate::version::FeatureVersion;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::{BinaryData, Identifier};
use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use crate::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;

impl From<BatchTransitionV1> for StateTransition {
    fn from(value: BatchTransitionV1) -> Self {
        let document_batch_transition: BatchTransition = value.into();
        document_batch_transition.into()
    }
}

impl StateTransitionLike for BatchTransitionV1 {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        self.transitions
            .iter()
            .filter_map(|t| match t {
                BatchedTransition::Document(document_transition) => {
                    Some(document_transition.base().id())
                }
                BatchedTransition::Token(_) => None,
            })
            .collect()
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        Batch
    }

    /// We create a list of unique identifiers for the batch
    ///
    /// Each transition contributes the owner, contract and nonce that replay-protect it. A batch
    /// that spends notes from a token's shielded pool contributes each spent nullifier as well,
    /// because two independent batches naming one nullifier cannot both execute — whichever lands
    /// records it and the other is refused on it — and their nonces need not be alike, so without
    /// the nullifier nothing about them collides. The credit pool's shielded transitions identify
    /// themselves by their nullifiers alone; these are the same values in the same encoding.
    ///
    /// Sharing a nullifier does not always mean mutual exclusion, so a consumer must not read it
    /// that way: a group action's proposer and its confirmers carry the same bundle, and are meant
    /// to. The values are also not namespaced by token, so two pools' nullifiers are compared as
    /// if they were one pool's. Neither matters while only the first identifier is used, and both
    /// have to be settled before any consumer looks past it.
    ///
    /// The nonce keys stay in front. Only the first identifier reaches the mempool today, and
    /// that one has to remain the key that stops a second batch replaying a nonce.
    fn unique_identifiers(&self) -> Vec<String> {
        let mut identifiers: Vec<String> = self
            .transitions
            .iter()
            .map(|transition| match transition {
                BatchedTransition::Document(document_transition) => {
                    format!(
                        "{}-{}-{:x}",
                        BASE64_STANDARD.encode(self.owner_id),
                        BASE64_STANDARD.encode(document_transition.data_contract_id()),
                        document_transition.identity_contract_nonce()
                    )
                }
                BatchedTransition::Token(token_transition) => {
                    format!(
                        "{}-{}-{:x}",
                        BASE64_STANDARD.encode(self.owner_id),
                        BASE64_STANDARD.encode(token_transition.data_contract_id()),
                        token_transition.identity_contract_nonce()
                    )
                }
            })
            .collect();

        identifiers.extend(self.transitions.iter().flat_map(|transition| {
            let actions = match transition {
                BatchedTransition::Token(token_transition) => {
                    token_transition.shielded_pool_actions()
                }
                // A document whose token cost is paid out of the token's shielded pool spends
                // pool notes exactly as an unshield does.
                BatchedTransition::Document(document_transition) => document_transition
                    .base()
                    .token_payment_info_ref()
                    .as_ref()
                    .and_then(|info| info.shielded_payment())
                    .map(|payment| payment.actions.as_slice()),
            };
            actions
                .unwrap_or_default()
                .iter()
                .map(|action| hex::encode(action.nullifier))
        }));

        identifiers
    }
}

impl StateTransitionHasUserFeeIncrease for BatchTransitionV1 {
    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionSingleSigned for BatchTransitionV1 {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionOwned for BatchTransitionV1 {
    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }
}
