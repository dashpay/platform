//! What the reads of a contract's team actions share.

use crate::drive::contract::paths::{contract_other_path, CONTRACT_TEAM_ACTIONS_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Whether the contract has the team actions tree a page of its actions is read under: a
    /// contract none of whose document types sets `moderatorAbilities.deleteSettled`, or an id
    /// nobody has, reads as keeping none, and its page as empty.
    pub(super) fn contract_keeps_team_actions(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        self.grove_has_raw(
            (&contract_other_path(contract_id.as_slice())).into(),
            &[CONTRACT_TEAM_ACTIONS_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
