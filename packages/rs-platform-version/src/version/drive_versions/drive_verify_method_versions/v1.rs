use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyContractMethodVersions, DriveVerifyDocumentMethodVersions,
    DriveVerifyIdentityMethodVersions, DriveVerifyMethodVersions,
    DriveVerifySingleDocumentMethodVersions, DriveVerifyStateTransitionMethodVersions,
    DriveVerifySystemMethodVersions, DriveVerifyVoteMethodVersions,
};

pub const DRIVE_VERIFY_METHOD_VERSIONS_V1: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    contract: DriveVerifyContractMethodVersions {
        verify_contract: 0,
        verify_contract_history: 0,
    },
    document: DriveVerifyDocumentMethodVersions {
        verify_proof: 0,
        verify_proof_keep_serialized: 0,
        verify_start_at_document_in_proof: 0,
    },
    identity: DriveVerifyIdentityMethodVersions {
        verify_full_identities_by_public_key_hashes: 0,
        verify_full_identity_by_identity_id: 0,
        verify_full_identity_by_public_key_hash: 0,
        verify_identity_balance_for_identity_id: 0,
        verify_identity_balances_for_identity_ids: 0,
        verify_identity_id_by_public_key_hash: 0,
        verify_identity_ids_by_public_key_hashes: 0,
        verify_identity_keys_by_identity_id: 0,
        verify_identity_nonce: 0,
        verify_identity_contract_nonce: 0,
        verify_identities_contract_keys: 0,
        verify_identity_revision_for_identity_id: 0,
    },
    single_document: DriveVerifySingleDocumentMethodVersions {
        verify_proof: 0,
        verify_proof_keep_serialized: 0,
    },
    system: DriveVerifySystemMethodVersions {
        verify_epoch_infos: 0,
        verify_epoch_proposers: 0,
        verify_elements: 0,
        verify_total_credits_in_system: 0,
        verify_upgrade_state: 0,
        verify_upgrade_vote_status: 0,
    },
    voting: DriveVerifyVoteMethodVersions {
        verify_masternode_vote: 0,
        verify_start_at_contender_in_proof: 0,
        verify_vote_poll_votes_proof: 0,
        verify_identity_votes_given_proof: 0,
        verify_vote_poll_vote_state_proof: 0,
        verify_contests_proof: 0,
        verify_vote_polls_by_end_date_proof: 0,
        verify_specialized_balance: 0,
    },
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 0,
    },
};
