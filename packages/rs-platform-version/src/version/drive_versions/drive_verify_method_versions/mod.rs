use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyMethodVersions {
    pub contract: DriveVerifyContractMethodVersions,
    pub document: DriveVerifyDocumentMethodVersions,
    pub identity: DriveVerifyIdentityMethodVersions,
    pub single_document: DriveVerifySingleDocumentMethodVersions,
    pub system: DriveVerifySystemMethodVersions,
    pub voting: DriveVerifyVoteMethodVersions,
    pub state_transition: DriveVerifyStateTransitionMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyContractMethodVersions {
    pub verify_contract: FeatureVersion,
    pub verify_contract_history: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
    pub verify_start_at_document_in_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyIdentityMethodVersions {
    pub verify_full_identities_by_public_key_hashes: FeatureVersion,
    pub verify_full_identity_by_identity_id: FeatureVersion,
    pub verify_full_identity_by_public_key_hash: FeatureVersion,
    pub verify_identity_balance_for_identity_id: FeatureVersion,
    pub verify_identity_balances_for_identity_ids: FeatureVersion,
    pub verify_identity_id_by_public_key_hash: FeatureVersion,
    pub verify_identity_ids_by_public_key_hashes: FeatureVersion,
    pub verify_identity_keys_by_identity_id: FeatureVersion,
    pub verify_identity_nonce: FeatureVersion,
    pub verify_identity_contract_nonce: FeatureVersion,
    pub verify_identities_contract_keys: FeatureVersion,
    pub verify_identity_revision_for_identity_id: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyVoteMethodVersions {
    pub verify_masternode_vote: FeatureVersion,
    pub verify_start_at_contender_in_proof: FeatureVersion,
    pub verify_vote_poll_votes_proof: FeatureVersion,
    pub verify_identity_votes_given_proof: FeatureVersion,
    pub verify_vote_poll_vote_state_proof: FeatureVersion,
    pub verify_contests_proof: FeatureVersion,
    pub verify_vote_polls_by_end_date_proof: FeatureVersion,
    pub verify_specialized_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySystemMethodVersions {
    pub verify_epoch_infos: FeatureVersion,
    pub verify_epoch_proposers: FeatureVersion,
    pub verify_elements: FeatureVersion,
    pub verify_total_credits_in_system: FeatureVersion,
    pub verify_upgrade_state: FeatureVersion,
    pub verify_upgrade_vote_status: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySingleDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyStateTransitionMethodVersions {
    pub verify_state_transition_was_executed_with_proof: FeatureVersion,
}
