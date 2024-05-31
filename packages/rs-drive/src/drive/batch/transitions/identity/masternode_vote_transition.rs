use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{
    IdentityOperation, PrefundedSpecializedBalanceOperation,
};
use crate::drive::batch::{DriveOperation, IdentityOperationType};

use crate::drive::batch::drive_op_batch::PrefundedSpecializedBalanceOperationType;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

impl DriveHighLevelOperationConverter for MasternodeVoteTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let pro_tx_hash = self.pro_tx_hash();
        let nonce = self.nonce();
        let strength = self.vote_strength();
        let vote = self.vote_owned();
        let prefunded_specialized_balance_id = vote.specialized_balance_id()?.ok_or(Error::Protocol(ProtocolError::VoteError("vote does not have a specialized balance from where it can use to pay for processing (this should have been caught during validation)".to_string())))?;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id: pro_tx_hash.into_buffer(),
                nonce,
            }),
            IdentityOperation(IdentityOperationType::MasternodeCastVote {
                voter_pro_tx_hash: pro_tx_hash.to_buffer(),
                strength,
                vote,
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
}
