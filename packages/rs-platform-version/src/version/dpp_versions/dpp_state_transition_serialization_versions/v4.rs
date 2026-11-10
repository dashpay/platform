//! V4 (PV15): identical to V3 except that the contract update transition admits version 1, the
//! delta form (`DataContractUpdateTransitionV1`), which carries only what an update changes,
//! and builds it by default. Version 0, which carries the whole contract, stays admitted.

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::DPPStateTransitionSerializationVersions;
use versioned_feature_core::FeatureVersionBounds;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V4: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        contract_update_state_transition: FeatureVersionBounds {
            min_version: 0,
            max_version: 1, // changed: version 1 carries only what the update changes
            default_current_version: 1, // changed: an update built from the stored contract is a delta
        },
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V3
    };
