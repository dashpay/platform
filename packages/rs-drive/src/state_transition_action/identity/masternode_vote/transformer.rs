use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl MasternodeVoteTransitionAction {
    /// Transforms an owned `MasternodeVoteTransition` into a `MasternodeVoteTransitionAction`.
    ///
    /// # Parameters
    ///
    /// - `value`: The owned `MasternodeVoteTransition` to transform.
    /// - `masternode_strength`: The strength of the masternode, normal ones have 1, evonodes have 4
    /// - `drive`: A reference to the `Drive` instance.
    /// - `transaction`: The transaction argument.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transformed `MasternodeVoteTransitionAction`, or an `Error` if the transformation fails.
    pub fn transform_from_owned_transition(
        value: MasternodeVoteTransition,
        masternode_strength: u8,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            MasternodeVoteTransition::V0(v0) => Ok(
                MasternodeVoteTransitionActionV0::transform_from_owned_transition(
                    v0,
                    masternode_strength,
                    drive,
                    transaction,
                    platform_version,
                )?
                .into(),
            ),
        }
    }

    /// Transforms a borrowed `MasternodeVoteTransition` into a `MasternodeVoteTransitionAction`.
    ///
    /// # Parameters
    ///
    /// - `value`: A reference to the `MasternodeVoteTransition` to transform.
    /// - `masternode_strength`: The strength of the masternode, normal ones have 1, evonodes have 4
    /// - `drive`: A reference to the `Drive` instance.
    /// - `transaction`: The transaction argument.
    /// - `platform_version`: A reference to the platform version.
    ///
    /// # Returns
    ///
    /// A `Result` containing the transformed `MasternodeVoteTransitionAction`, or an `Error` if the transformation fails.
    pub fn transform_from_transition(
        value: &MasternodeVoteTransition,
        masternode_strength: u8,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            MasternodeVoteTransition::V0(v0) => {
                Ok(MasternodeVoteTransitionActionV0::transform_from_transition(
                    v0,
                    masternode_strength,
                    drive,
                    transaction,
                    platform_version,
                )?
                .into())
            }
        }
    }
}
