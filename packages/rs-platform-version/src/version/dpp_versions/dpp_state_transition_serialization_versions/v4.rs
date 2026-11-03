//! V4 (PV15): identical to V3 except `random_token_amount` is `Some(0)`: a batch carrying a
//! version 1 token transfer or mint, which can move a random amount, is active, as block
//! execution settles the amounts from the random beacon Tenderdash delivers from consensus
//! version 2 on (`settle_random_token_amounts`).

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::DPPStateTransitionSerializationVersions;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V4: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        // changed: version 1 token transfers and mints, which carry a random amount, decode
        random_token_amount: Some(0),
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V3
    };
