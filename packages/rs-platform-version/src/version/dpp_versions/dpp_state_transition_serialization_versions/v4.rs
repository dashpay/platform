//! V4 (PV15): V3 plus batch transition format 2 and the erase kind.
//!
//! Format 2 carries a document transition shell that knows the erase kind
//! (`DocumentEraseTransition`), which purges the retained revisions of a
//! deleted keep-history document. Formats 0 and 1 keep their shells, so the
//! kind cannot appear in them; `document_erase_state_transition` bounds the
//! kind's own generation and stays `None` in every earlier table.

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::{
    DPPStateTransitionSerializationVersions, DocumentFeatureVersionBounds,
};
use versioned_feature_core::FeatureVersionBounds;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V4: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        batch_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 2,
            default_current_version: 2,
        },
        document_erase_state_transition: Some(DocumentFeatureVersionBounds {
            bounds: FeatureVersionBounds {
                min_version: 0,
                max_version: 0,
                default_current_version: 0,
            },
        }),
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V3
    };
