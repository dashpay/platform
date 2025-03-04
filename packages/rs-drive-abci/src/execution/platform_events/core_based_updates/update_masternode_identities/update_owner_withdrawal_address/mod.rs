use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the owner's withdrawal address.
    ///
    /// This function is responsible for updating the withdrawal address of the owner.
    /// The withdrawal address is a key attribute for any owner in a masternode.
    ///
    /// The change is first validated and then reflected on the blockchain.
    /// The function is implemented as a versioned function to allow changes to the
    /// implementation over time while maintaining backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `masternode`: a tuple of ProTxHash and DMNStateDiff containing the masternode details and state difference.
    /// * `block_info`: an object containing information about the block where this operation is happening.
    /// * `transaction`: the transaction in which this operation is happening.
    /// * `drive_operations`: a mutable reference to a vector of DriveOperations.
    /// * `platform_version`: the current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is no existing withdrawal address or
    /// if the existing withdrawal address is already disabled.
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn update_owner_withdrawal_address(
        &self,
        owner_identifier: [u8; 32],
        new_withdrawal_address: [u8; 20],
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .update_owner_withdrawal_address
        {
            0 => self.update_owner_withdrawal_address_v0(
                owner_identifier,
                new_withdrawal_address,
                block_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            1 => self.update_owner_withdrawal_address_v1(
                owner_identifier,
                new_withdrawal_address,
                block_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_owner_withdrawal_address".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::identity::{Identity, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    fn test_update_withdrawal_address() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let payout_address: [u8; 20] = rng.gen();

        let withdrawal_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([(0, withdrawal_key.clone())]),
            balance: 0,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_owner_withdrawal_address(
                identity.id().to_buffer(),
                [0; 20],
                &block_info,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update owner withdrawal address");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }

    #[test]
    fn test_update_to_same_withdrawal_address() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let payout_address: [u8; 20] = rng.gen();

        let withdrawal_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([(0, withdrawal_key.clone())]),
            balance: 0,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_owner_withdrawal_address(
                identity.id().to_buffer(),
                payout_address,
                &block_info,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update owner withdrawal address");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }
    #[test]
    fn test_update_to_previously_disabled_withdrawal_address() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_info = BlockInfo::default();

        let mut rng = StdRng::seed_from_u64(5);

        let payout_address: [u8; 20] = rng.gen();

        let withdrawal_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into();

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([(0, withdrawal_key.clone())]),
            balance: 0,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                true,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_owner_withdrawal_address(
                identity.id().to_buffer(),
                [0; 20],
                &block_info,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update owner withdrawal address");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");

        let transaction = platform.drive.grove.start_transaction();

        let mut drive_operations = vec![];

        platform
            .update_owner_withdrawal_address(
                identity.id().to_buffer(),
                payout_address,
                &block_info,
                &transaction,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to update owner withdrawal address");

        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply drive operations");
    }
}
