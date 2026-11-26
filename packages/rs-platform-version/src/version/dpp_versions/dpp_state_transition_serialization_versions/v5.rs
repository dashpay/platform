//! V5 (PV16): V4 plus random token amounts.
//!
//! `random_token_amount` is `Some(0)`: a batch carrying a version 1 token transfer or mint,
//! which can move a random amount, is active, as block execution settles the amounts from the
//! random beacon Tenderdash delivers from consensus version 3 on
//! (`settle_random_token_amounts`).

use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v4::STATE_TRANSITION_SERIALIZATION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::DPPStateTransitionSerializationVersions;

pub const STATE_TRANSITION_SERIALIZATION_VERSIONS_V5: DPPStateTransitionSerializationVersions =
    DPPStateTransitionSerializationVersions {
        random_token_amount: Some(0), // changed: version 1 token transfers and mints decode
        ..STATE_TRANSITION_SERIALIZATION_VERSIONS_V4
    };
