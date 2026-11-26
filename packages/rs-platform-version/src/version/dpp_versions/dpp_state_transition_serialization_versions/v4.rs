//! V4 (PV15): V3 plus the erase kind.
//!
//! `DocumentEraseTransition` purges the retained revisions of a deleted
//! keep-history document. The kind is appended to the batch's document
//! transition enum; `document_erase_state_transition` bounds its generation
//! and stays `None` in every earlier table, where the batch basic-structure
//! gate refuses the kind.

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
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V3
    };
