use crate::version::dpp_versions::dpp_state_transition_method_versions::v2::STATE_TRANSITION_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::DPPStateTransitionMethodVersions;

/// V3 is protocol version 15's table. `verify_hash_signature` 0 -> 1: a signature checked by
/// recovering its public key (an ECDSA_HASH160 identity key, a P2PKH address witness, an asset
/// lock's one-time key) must have S in the lower half of the curve order, so a third party can no
/// longer turn a valid signature into a second one and change the transition's hash. V2 stays as
/// is for replay.
pub const STATE_TRANSITION_METHOD_VERSIONS_V3: DPPStateTransitionMethodVersions =
    DPPStateTransitionMethodVersions {
        verify_hash_signature: 1,
        ..STATE_TRANSITION_METHOD_VERSIONS_V2
    };
