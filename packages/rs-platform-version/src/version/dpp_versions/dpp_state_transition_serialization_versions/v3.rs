//! V3 (PV14): identical to V2 except the new
//! `document_index_only_delete_state_transition` kind exists (`Some`, V0
//! only) — the indexOnly delete-by-values transition
//! (`DocumentIndexOnlyDeleteTransition`). It is its own kind, not a
//! version of the delete transition: `document_delete_state_transition`
//! keeps V2's bounds and keeps evolving independently for stored types.

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::{
    DPPStateTransitionSerializationVersions, DocumentFeatureVersionBounds,
};
use versioned_feature_core::FeatureVersionBounds;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V3: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        identity_public_key_in_creation: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_create_from_addresses_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_create_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_update_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_top_up_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_top_up_from_addresses_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_credit_withdrawal_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_credit_transfer_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_credit_transfer_to_addresses_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        masternode_vote_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        contract_create_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        contract_update_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        batch_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 1,
            default_current_version: 1,
        },
        document_base_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 1,
        },
        document_create_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        document_replace_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        document_delete_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        document_index_only_delete_state_transition: Some(DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        }),
        document_transfer_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        document_update_price_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        document_purchase_state_transition: DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        },
        address_funds_transfer_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        address_funding_from_asset_lock_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        address_credit_withdrawal_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        shield_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        shielded_transfer_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        unshield_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        shield_from_asset_lock_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        shielded_withdrawal_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
        identity_create_from_shielded_pool_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 0,
            default_current_version: 0,
        },
    };
