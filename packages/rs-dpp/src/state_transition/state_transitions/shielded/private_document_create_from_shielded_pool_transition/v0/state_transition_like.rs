use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;

impl From<PrivateDocumentCreateFromShieldedPoolTransitionV0> for StateTransition {
    fn from(value: PrivateDocumentCreateFromShieldedPoolTransitionV0) -> Self {
        let transition: PrivateDocumentCreateFromShieldedPoolTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for PrivateDocumentCreateFromShieldedPoolTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::PrivateDocumentCreateFromShieldedPool
    }

    /// No identity or document is modified: the entries are appended to the type's store,
    /// which takes any number of them.
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// Unique by spent nullifier, as every pool-paid transition: two transitions spending the
    /// same note can never both be valid.
    fn unique_identifiers(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| hex::encode(action.nullifier))
            .collect()
    }
}
