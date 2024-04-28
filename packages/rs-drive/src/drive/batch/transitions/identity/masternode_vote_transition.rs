use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::IdentityOperation;
use crate::drive::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;

impl DriveHighLevelOperationConverter for MasternodeVoteTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let pro_tx_hash = self.pro_tx_hash();
        let nonce = self.nonce();
        let vote = self.vote_owned();

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id: pro_tx_hash.into_buffer(),
                nonce,
            }),
            IdentityOperation(IdentityOperationType::MasternodeCastVote {
                voter_pro_tx_hash: pro_tx_hash.to_buffer(),
                vote,
            }),
        ];
        Ok(drive_operations)
    }
}
