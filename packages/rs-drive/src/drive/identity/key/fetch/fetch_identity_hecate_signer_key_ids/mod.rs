mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the ids of an identity's enabled Hecate signer keys from its signer index, at most
    /// `limit` of them in key id order, with the fee of the read so that consensus validation
    /// can bill it.
    ///
    /// The index holds a reference per enabled `HECATE_SIGNER` key: adding the key writes it
    /// and disabling the key removes it. An identity that never had one has no index, and
    /// reads none.
    ///
    /// # Parameters
    /// - `identity_id`: The identity, which must exist.
    /// - `limit`: At most this many key ids.
    /// - `epoch`: The epoch the fee is priced for.
    /// - `transaction`: The transaction to read in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - `Ok((FeeResult, Vec<KeyID>))` with the fee and the key ids.
    /// - `Err(Error)` when the method is not active at this version, or the read fails.
    pub fn fetch_identity_hecate_signer_key_ids_with_fee(
        &self,
        identity_id: [u8; 32],
        limit: u16,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Vec<KeyID>), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let key_ids = self.fetch_identity_hecate_signer_key_ids_operations(
            identity_id,
            limit,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((fee, key_ids))
    }

    /// [`Drive::fetch_identity_hecate_signer_key_ids_with_fee`], adding the cost of the read to
    /// `drive_operations`. The read is always stateful.
    pub(crate) fn fetch_identity_hecate_signer_key_ids_operations(
        &self,
        identity_id: [u8; 32],
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<KeyID>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .fetch
            .fetch_identity_hecate_signer_key_ids
        {
            Some(0) => self.fetch_identity_hecate_signer_key_ids_operations_v0(
                identity_id,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_hecate_signer_key_ids_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_identity_hecate_signer_key_ids_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::version::PlatformVersion;
    use grovedb::TransactionArg;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use crate::drive::Drive;

    fn key(id: KeyID, purpose: Purpose, key_type: KeyType, seed: u64) -> IdentityPublicKey {
        let mut rng = StdRng::seed_from_u64(seed);
        IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: if purpose == Purpose::AUTHENTICATION {
                SecurityLevel::HIGH
            } else {
                SecurityLevel::MEDIUM
            },
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(
                key_type
                    .random_public_key_data(&mut rng, PlatformVersion::latest())
                    .expect("expected random key data"),
            ),
            disabled_at: None,
        }
        .into()
    }

    fn signer_key(id: KeyID, seed: u64) -> IdentityPublicKey {
        key(id, Purpose::HECATE_SIGNER, KeyType::REDPALLAS_HECATE, seed)
    }

    /// An identity with three random keys, stored; returns its id.
    fn stored_identity(drive: &Drive, seed: u64) -> [u8; 32] {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(3, Some(seed), platform_version)
            .expect("expected a random identity");
        let identity_id = identity.id().to_buffer();
        drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the identity");
        identity_id
    }

    fn add_keys(drive: &Drive, identity_id: [u8; 32], keys: Vec<IdentityPublicKey>) {
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                keys,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to add the keys");
    }

    fn signer_key_ids(drive: &Drive, identity_id: [u8; 32]) -> (FeeResult, Vec<KeyID>) {
        signer_key_ids_at_most(drive, identity_id, 10, None)
    }

    fn signer_key_ids_at_most(
        drive: &Drive,
        identity_id: [u8; 32],
        limit: u16,
        transaction: TransactionArg,
    ) -> (FeeResult, Vec<KeyID>) {
        drive
            .fetch_identity_hecate_signer_key_ids_with_fee(
                identity_id,
                limit,
                &BlockInfo::default().epoch,
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to read the signer index")
    }

    #[test]
    fn should_hold_only_the_enabled_hecate_signer_keys() {
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = stored_identity(&drive, 1);

        // No signer key yet: the identity has no signer index, and reads none.
        assert_eq!(signer_key_ids(&drive, identity_id).1, Vec::<KeyID>::new());

        add_keys(&drive, identity_id, vec![signer_key(3, 10)]);
        assert_eq!(signer_key_ids(&drive, identity_id).1, vec![3]);

        drive
            .disable_identity_keys(
                identity_id,
                vec![3],
                1,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to disable the signer key");
        assert_eq!(signer_key_ids(&drive, identity_id).1, Vec::<KeyID>::new());

        add_keys(&drive, identity_id, vec![signer_key(4, 11)]);
        assert_eq!(signer_key_ids(&drive, identity_id).1, vec![4]);
    }

    #[test]
    fn should_read_a_signer_key_added_in_the_transaction_it_reads_in() {
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = stored_identity(&drive, 7);
        let transaction = drive.grove.start_transaction();

        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![signer_key(3, 40)],
                &BlockInfo::default(),
                true,
                Some(&transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to add the signer key in the transaction");

        // Uncommitted: the transaction reads the key, committed state doesn't.
        assert_eq!(
            signer_key_ids_at_most(&drive, identity_id, 10, Some(&transaction)).1,
            vec![3]
        );
        assert_eq!(signer_key_ids(&drive, identity_id).1, Vec::<KeyID>::new());
    }

    #[test]
    fn should_read_the_signer_index_at_a_cost_that_does_not_grow_with_the_identity_keys() {
        let drive = setup_drive_with_initial_state_structure(None);

        let few_keys = stored_identity(&drive, 2);
        add_keys(&drive, few_keys, vec![signer_key(3, 20)]);

        let many_keys = stored_identity(&drive, 3);
        add_keys(
            &drive,
            many_keys,
            (3..40)
                .map(|id| {
                    key(
                        id,
                        Purpose::AUTHENTICATION,
                        KeyType::ECDSA_SECP256K1,
                        100 + id as u64,
                    )
                })
                .collect(),
        );
        add_keys(&drive, many_keys, vec![signer_key(40, 21)]);

        let (few_keys_fee, few_keys_ids) = signer_key_ids(&drive, few_keys);
        let (many_keys_fee, many_keys_ids) = signer_key_ids(&drive, many_keys);
        assert_eq!(few_keys_ids, vec![3]);
        assert_eq!(many_keys_ids, vec![40]);
        // Equal fees prove nothing if the read is not billed at all.
        assert!(few_keys_fee.processing_fee > 0, "the read must be billed");
        assert_eq!(
            few_keys_fee.processing_fee, many_keys_fee.processing_fee,
            "the read must not grow with the identity's other keys"
        );
    }

    fn root_hash(drive: &Drive) -> [u8; 32] {
        drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    }

    /// Estimate `operation` (`apply` false), check the estimate wrote nothing,
    /// then run it and check the estimate covers what the run cost.
    fn assert_estimate_covers_execution(
        drive: &Drive,
        what: &str,
        operation: impl Fn(bool) -> FeeResult,
    ) {
        let before = root_hash(drive);
        let estimated = operation(false);
        assert_eq!(root_hash(drive), before, "estimating {what} must not write");
        let actual = operation(true);
        assert_ne!(root_hash(drive), before, "{what} must write");
        assert!(
            estimated.processing_fee >= actual.processing_fee,
            "{what}: estimated processing {} must cover execution {}",
            estimated.processing_fee,
            actual.processing_fee
        );
        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "{what}: estimated storage {} must cover execution {}",
            estimated.storage_fee,
            actual.storage_fee
        );
    }

    fn add_keys_fee(
        drive: &Drive,
        identity_id: [u8; 32],
        keys: &[IdentityPublicKey],
        apply: bool,
    ) -> FeeResult {
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                keys.to_vec(),
                &BlockInfo::default(),
                apply,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to add the keys")
    }

    fn disable_keys_fee(
        drive: &Drive,
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        apply: bool,
    ) -> FeeResult {
        drive
            .disable_identity_keys(
                identity_id,
                key_ids.to_vec(),
                1,
                &BlockInfo::default(),
                apply,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to disable the keys")
    }

    #[test]
    fn should_estimate_a_first_signer_key_at_least_at_its_execution_cost() {
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = stored_identity(&drive, 4);
        let keys = [signer_key(3, 30)];

        // The identity has no signer index yet, so the add creates it.
        assert_estimate_covers_execution(&drive, "the first signer key", |apply| {
            add_keys_fee(&drive, identity_id, &keys, apply)
        });
        assert_eq!(signer_key_ids(&drive, identity_id).1, vec![3]);
    }

    #[test]
    fn should_estimate_disabling_a_signer_key_at_least_at_its_execution_cost() {
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = stored_identity(&drive, 5);
        add_keys(&drive, identity_id, vec![signer_key(3, 31)]);

        assert_estimate_covers_execution(&drive, "disabling the only signer key", |apply| {
            disable_keys_fee(&drive, identity_id, &[3], apply)
        });
        assert_eq!(signer_key_ids(&drive, identity_id).1, Vec::<KeyID>::new());
    }

    #[test]
    fn should_estimate_rotating_a_signer_key_through_the_index_at_least_at_its_execution_cost() {
        let drive = setup_drive_with_initial_state_structure(None);
        let identity_id = stored_identity(&drive, 6);
        add_keys(&drive, identity_id, vec![signer_key(3, 32)]);
        let replacement = [signer_key(4, 33)];

        // The new key joins the index that already holds key 3, then key 3
        // leaves it while key 4 stays.
        assert_estimate_covers_execution(&drive, "a second signer key", |apply| {
            add_keys_fee(&drive, identity_id, &replacement, apply)
        });
        assert_eq!(signer_key_ids(&drive, identity_id).1, vec![3, 4]);
        // The read stops at its limit, in key id order.
        assert_eq!(
            signer_key_ids_at_most(&drive, identity_id, 1, None).1,
            vec![3]
        );
        assert_estimate_covers_execution(&drive, "disabling the replaced signer key", |apply| {
            disable_keys_fee(&drive, identity_id, &[3], apply)
        });
        assert_eq!(signer_key_ids(&drive, identity_id).1, vec![4]);
    }
}
