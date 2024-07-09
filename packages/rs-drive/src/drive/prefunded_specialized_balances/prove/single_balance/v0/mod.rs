use crate::drive::Drive;
use crate::error::Error;

use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path_vec;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    /// Proves the prefunded specialized balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn prove_prefunded_specialized_balance_v0(
        &self,
        balance_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let balance_path = prefunded_specialized_balances_for_voting_path_vec();
        let balance_query = PathQuery::new_single_key(balance_path, balance_id.to_vec());
        self.grove_get_proved_path_query(
            &balance_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
