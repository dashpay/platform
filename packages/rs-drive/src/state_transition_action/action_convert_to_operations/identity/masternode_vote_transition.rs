use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{IdentityOperation, PrefundedSpecializedBalanceOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use crate::util::batch::drive_op_batch::PrefundedSpecializedBalanceOperationType;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

impl DriveHighLevelOperationConverter for MasternodeVoteTransitionAction {
    fn into_high_level_drive_operations<'a>(
        mut self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .masternode_vote_transition
        {
            0 => {
                let voter_identity_id = self.voter_identity_id();
                let pro_tx_hash = self.pro_tx_hash();
                let nonce = self.nonce();
                let strength = self.vote_strength();
                let previous_resource_vote_choice_to_remove =
                    self.take_previous_resource_vote_choice_to_remove();
                let vote = self.vote_owned();
                let prefunded_specialized_balance_id = vote.specialized_balance_id()?.ok_or(Error::Protocol(ProtocolError::VoteError("vote does not have a specialized balance from where it can use to pay for processing (this should have been caught during validation)".to_string())))?;

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: voter_identity_id.into_buffer(),
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::MasternodeCastVote {
                        // Votes are cast based on masternode pro_tx_hash, and not the voter identity id
                        voter_pro_tx_hash: pro_tx_hash.to_buffer(),
                        strength,
                        vote,
                        previous_resource_vote_choice_to_remove,
                    }),
                    // Casting a vote has a fixed cost
                    PrefundedSpecializedBalanceOperation(
                        PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                            prefunded_specialized_balance_id,
                            remove_balance: platform_version
                                .fee_version
                                .vote_resolution_fund_fees
                                .contested_document_single_vote_cost,
                        },
                    ),
                ];
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "MasternodeVoteTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
