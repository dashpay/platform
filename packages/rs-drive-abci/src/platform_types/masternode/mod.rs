use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::masternode::v0::MasternodeV0;
use bincode::{Decode, Encode};
use dpp::dashcore_rpc::json::{DMNState, DMNStateDiff, MasternodeListItem};
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};

mod accessors;
/// Version 0
pub mod v0;

/// `Masternode` represents a masternode on the network.
#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Masternode {
    /// Version 0
    V0(MasternodeV0),
}

impl TryFromPlatformVersioned<MasternodeListItem> for Masternode {
    type Error = Error;

    fn try_from_platform_versioned(
        value: MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version.drive_abci.structs.masternode {
            0 => Ok(Self::V0(value.into())),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Masternode::try_from_platform_versioned(MasternodeListItem)".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl From<Masternode> for MasternodeListItem {
    fn from(value: Masternode) -> Self {
        match value {
            Masternode::V0(v0) => v0.into(),
        }
    }
}

/// Applies a Core diff to a masternode state with the revived height replaced by the diff's, so
/// a diff that does not print `PoSeRevivedHeight` clears it.
///
/// Core prints the revived height only in a diff that spans the revive, so this loses it on the
/// masternode's next diff. The masternode list generations calling this
/// (`update_state_masternode_list` v0 and v1) have always applied diffs this way, and their
/// stored lists and validator sets for every processed block follow from it, so they keep doing
/// so. A generation that keeps Core's revived height must apply diffs another way.
pub(crate) fn apply_diff_replacing_revived_height(state: &mut DMNState, diff: DMNStateDiff) {
    let pose_revived_height = diff.pose_revived_height;
    state.apply_diff(diff);
    state.pose_revived_height = pose_revived_height;
}

#[cfg(test)]
mod tests {
    use super::apply_diff_replacing_revived_height;
    use dpp::dashcore_rpc::json::{DMNState, DMNStateDiff};
    use serde_json::json;

    fn state_with_revived_height(pose_revived_height: Option<u32>) -> DMNState {
        let mut state: DMNState = serde_json::from_value(json!({
            "service": "192.0.2.1:9999",
            "registeredHeight": 1000,
            "PoSeRevivedHeight": -1,
            "PoSeBanHeight": -1,
            "revocationReason": 0,
            "ownerAddress": "yPxSLV3rNkiq4aNsvYHLb5RzgtwwnHPLWP",
            "votingAddress": "yPxSLV3rNkiq4aNsvYHLb5RzgtwwnHPLWP",
            "payoutAddress": "yPxSLV3rNkiq4aNsvYHLb5RzgtwwnHPLWP",
            "pubKeyOperator": "00".repeat(48),
        }))
        .expect("expected a masternode state");
        state.pose_revived_height = pose_revived_height;
        state
    }

    fn diff(json: serde_json::Value) -> DMNStateDiff {
        serde_json::from_value(json).expect("expected a Core state diff")
    }

    #[test]
    fn should_clear_a_revived_height_the_diff_does_not_print() {
        let mut state = state_with_revived_height(Some(1100));
        apply_diff_replacing_revived_height(&mut state, diff(json!({"lastPaidHeight": 1200})));
        assert_eq!(state.pose_revived_height, None);
    }

    #[test]
    fn should_take_the_revived_height_the_diff_prints() {
        for previous in [None, Some(1100)] {
            let mut state = state_with_revived_height(previous);
            apply_diff_replacing_revived_height(
                &mut state,
                diff(json!({"PoSeRevivedHeight": 1200, "PoSeBanHeight": -1})),
            );
            assert_eq!(state.pose_revived_height, Some(1200), "from {previous:?}");
        }
    }
}
