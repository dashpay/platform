use versioned_feature_core::FeatureVersionBounds;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionSerializationVersions {
    pub identity_public_key_in_creation: FeatureVersionBounds,
    pub identity_create_from_addresses_state_transition: FeatureVersionBounds,
    pub identity_create_state_transition: FeatureVersionBounds,
    pub identity_update_state_transition: FeatureVersionBounds,
    pub identity_top_up_state_transition: FeatureVersionBounds,
    pub identity_top_up_from_addresses_state_transition: FeatureVersionBounds,
    pub identity_credit_withdrawal_state_transition: FeatureVersionBounds,
    pub identity_credit_transfer_state_transition: FeatureVersionBounds,
    pub identity_credit_transfer_to_addresses_state_transition: FeatureVersionBounds,
    pub masternode_vote_state_transition: FeatureVersionBounds,
    pub contract_create_state_transition: FeatureVersionBounds,
    pub contract_update_state_transition: FeatureVersionBounds,
    pub batch_state_transition: FeatureVersionBounds,
    pub document_base_state_transition: FeatureVersionBounds,
    pub document_create_state_transition: DocumentFeatureVersionBounds,
    pub document_replace_state_transition: DocumentFeatureVersionBounds,
    pub document_delete_state_transition: DocumentFeatureVersionBounds,
    pub document_transfer_state_transition: DocumentFeatureVersionBounds,
    pub document_update_price_state_transition: DocumentFeatureVersionBounds,
    pub document_purchase_state_transition: DocumentFeatureVersionBounds,
    pub address_funds_transfer_state_transition: FeatureVersionBounds,
    pub address_funding_from_asset_lock_state_transition: FeatureVersionBounds,
    pub address_credit_withdrawal_state_transition: FeatureVersionBounds,
    pub shield_state_transition: FeatureVersionBounds,
    pub shielded_transfer_state_transition: FeatureVersionBounds,
    pub unshield_state_transition: FeatureVersionBounds,
    pub shield_from_asset_lock_state_transition: FeatureVersionBounds,
    pub shielded_withdrawal_state_transition: FeatureVersionBounds,
    pub identity_create_from_shielded_pool_state_transition: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentFeatureVersionBounds {
    pub bounds: FeatureVersionBounds,
}
