use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionVersions {
    pub documents: DocumentTransitionVersions,
    pub identities: IdentityTransitionVersions,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityTransitionVersions {
    pub max_public_keys_in_creation: u16,
    pub asset_locks: IdentityTransitionAssetLockVersions,
    pub credit_withdrawal: IdentityCreditWithdrawalTransitionVersions,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityCreditWithdrawalTransitionVersions {
    pub default_constructor: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityTransitionAssetLockVersions {
    pub required_asset_lock_duff_balance_for_processing_start_for_identity_create: u64,
    pub required_asset_lock_duff_balance_for_processing_start_for_identity_top_up: u64,
    pub validate_asset_lock_transaction_structure: FeatureVersion,
    pub validate_instant_asset_lock_proof_structure: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTransitionVersions {
    pub documents_batch_transition: DocumentsBatchTransitionVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentsBatchTransitionVersions {
    pub validation: DocumentsBatchTransitionValidationVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentsBatchTransitionValidationVersions {
    pub find_duplicates_by_id: FeatureVersion,
    pub validate_base_structure: FeatureVersion,
}