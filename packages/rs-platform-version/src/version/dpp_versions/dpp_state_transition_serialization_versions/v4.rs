//! V4 (PV15): V3 plus the erase kind and random token amounts.
//!
//! `DocumentEraseTransition` purges the retained revisions of a deleted
//! keep-history document. The kind is appended to the batch's document
//! transition enum; `document_erase_state_transition` bounds its generation
//! and stays `None` in every earlier table, where the batch basic-structure
//! gate refuses the kind.
//!
//! `random_token_amount` is `Some(0)`: a batch carrying a version 1 token
//! transfer or mint, which can move a random amount, is active, as block
//! execution settles the amounts from the random beacon Tenderdash delivers
//! from consensus version 2 on (`settle_random_token_amounts`).

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::{
    DPPStateTransitionSerializationVersions, DocumentFeatureVersionBounds,
};
use versioned_feature_core::FeatureVersionBounds;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V4: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        document_erase_state_transition: Some(DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        }),
        // changed: version 1 token transfers and mints, which carry a random amount, decode
        random_token_amount: Some(0),
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V3
    };
