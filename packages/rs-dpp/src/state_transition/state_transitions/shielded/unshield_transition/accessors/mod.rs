mod v0;

pub use v0::*;

use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::state_transition::unshield_transition::UnshieldTransition;

impl UnshieldTransitionAccessorsV0 for UnshieldTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            UnshieldTransition::V0(v0) => &v0.actions,
        }
    }

    fn output_address(&self) -> &PlatformAddress {
        match self {
            UnshieldTransition::V0(v0) => &v0.output_address,
        }
    }
}
