use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::v0::{
    MasternodeVoteTransitionActionV0, PreviousVoteCount,
};
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl MasternodeVoteTransitionAction {
    /// Transforms an owned `MasternodeVoteTransition` into a `MasternodeVoteTransitionAction`.
    ///
    /// # Parameters
    ///
    /// - `value`: The owned `MasternodeVoteTransition` to transform.
    /// - `voter_identity_id`: The pre-calculated voter identity id, if it isn't given we will calculate it
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
        voting_address: [u8; 20],
        masternode_strength: u8,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            MasternodeVoteTransition::V0(v0) => Ok(
                MasternodeVoteTransitionActionV0::transform_from_owned_transition(
                    v0,
                    voting_address,
                    masternode_strength,
                    previous_resource_vote_choice_to_remove,
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
    /// - `voter_identity_id`: The pre-calculated voter identity id, if it isn't given we will calculate it
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
        voting_address: [u8; 20],
        masternode_strength: u8,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match value {
            MasternodeVoteTransition::V0(v0) => {
                Ok(MasternodeVoteTransitionActionV0::transform_from_transition(
                    v0,
                    voting_address,
                    masternode_strength,
                    previous_resource_vote_choice_to_remove,
                    drive,
                    transaction,
                    platform_version,
                )?
                .into())
            }
        }
    }
}
