use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::v0::PrivateDocumentCreateFromShieldedPoolTransitionActionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use std::sync::Arc;

impl PrivateDocumentCreateFromShieldedPoolTransitionActionV0 {
    /// Builds the v0 action from the v0 transition, the contract and the store's chunk power
    /// the transform read, the pool total, and the fee
    pub fn try_from_transition(
        value: &PrivateDocumentCreateFromShieldedPoolTransitionV0,
        data_contract: Arc<DataContractFetchInfo>,
        store_chunk_power: u8,
        current_total_balance: Credits,
        fee_amount: Credits,
    ) -> ConsensusValidationResult<Self> {
        let notes: Vec<ShieldedActionNote> =
            value.actions.iter().map(ShieldedActionNote::from).collect();

        ConsensusValidationResult::new_with_data(
            PrivateDocumentCreateFromShieldedPoolTransitionActionV0 {
                data_contract,
                document_type_name: value.document_type_name.clone(),
                entries: value.entries.iter().map(|entry| entry.to_vec()).collect(),
                store_chunk_power,
                notes,
                anchor: value.anchor,
                fee_amount,
                current_total_balance,
            },
        )
    }
}
