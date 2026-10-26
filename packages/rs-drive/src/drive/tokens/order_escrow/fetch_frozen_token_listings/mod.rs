use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::tokens::token_order::TokenOrder;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

mod v0;

impl Drive {
    /// The open listings of `owner_id` for `token_id` that a `TokenDestroyFrozenFunds` destroys
    /// with the frozen balance: the oldest first, at most
    /// `max_frozen_listings_destroyed_per_transition`, read through the `ask` type's `byOwner`
    /// index, with the processing cost of the read billed at `epoch`. `VersionNotActive` where
    /// `fetch_frozen_listings` is `None` (before protocol version 15): a destroy there reads no
    /// listings and does not call this.
    pub fn fetch_frozen_token_listings(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<TokenOrder>, u64), Error> {
        match platform_version
            .drive
            .methods
            .token
            .order_escrow
            .fetch_frozen_listings
        {
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_frozen_token_listings".to_string(),
                known_versions: vec![0],
            })),
            Some(0) => self.fetch_frozen_token_listings_v0(
                token_id,
                owner_id,
                epoch,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_frozen_token_listings".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    /// Before protocol version 15 the read is not active: it fails closed rather than answering
    /// with no listings, on a chain with no marketplace state (no escrow trees, no token market
    /// contract).
    #[test]
    fn should_refuse_to_read_frozen_listings_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let result = drive.fetch_frozen_token_listings(
            Identifier::from([5; 32]),
            Identifier::from([6; 32]),
            &Epoch::new(0).expect("expected an epoch"),
            None,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::VersionNotActive { .. }))
            ),
            "{result:?}"
        );
    }
}
