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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn prove_nonexistent_balance_returns_non_empty_proof() {
        // Proving a non-existent key must still produce a non-empty proof
        // (the proof attests absence). It must not error.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let proof = drive
            .prove_prefunded_specialized_balance_v0([0xAAu8; 32], None, platform_version)
            .expect("prove non-existent should succeed");
        assert!(!proof.is_empty());
    }

    #[test]
    fn prove_existing_balance_returns_non_empty_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([77u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 100, None, platform_version)
            .expect("seed");

        let proof = drive
            .prove_prefunded_specialized_balance_v0(id.to_buffer(), None, platform_version)
            .expect("prove existing");
        assert!(!proof.is_empty());
    }
}
