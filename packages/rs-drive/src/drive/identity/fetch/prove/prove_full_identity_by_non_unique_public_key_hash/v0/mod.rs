use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Builds a read-only lookup proof; no block-execution or stored-state behavior changes.
    pub(super) fn prove_full_identity_by_non_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_full_identity_by_non_unique_public_key_hash_v0_operations(
            public_key_hash,
            after,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    fn prove_full_identity_by_non_unique_public_key_hash_v0_operations(
        &self,
        public_key_hash: [u8; 20],
        after: Option<[u8; 32]>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        const MAX_ATTEMPTS: usize = 3;
        for _ in 0..MAX_ATTEMPTS {
            let root_before = self
                .grove
                .root_hash(None, &platform_version.drive.grove_version);
            drive_operations.push(LowLevelDriveOperation::CalculatedCostOperation(
                root_before.cost,
            ));
            let root_before = root_before.value?;
            let result = (|| {
                let identity_ids = self
                    .fetch_identity_ids_by_non_unique_public_key_hash_operations(
                        public_key_hash,
                        Some(1),
                        after,
                        transaction,
                        drive_operations,
                        platform_version,
                    )?;
                #[cfg(test)]
                selection_hook::run(self);
                let query = match identity_ids.first() {
                    Some(identity_id) => Self::full_identity_with_non_unique_public_key_hash_query(
                        public_key_hash,
                        *identity_id,
                        after,
                        &platform_version.drive.grove_version,
                    )?,
                    None => Self::identity_id_by_non_unique_public_key_hash_query(
                        public_key_hash,
                        after,
                    ),
                };
                self.grove_get_proved_path_query(
                    &query,
                    transaction,
                    drive_operations,
                    &platform_version.drive,
                )
            })();
            let root_after = self
                .grove
                .root_hash(None, &platform_version.drive.grove_version);
            drive_operations.push(LowLevelDriveOperation::CalculatedCostOperation(
                root_after.cost,
            ));
            let root_after = match root_after.value {
                Ok(root) => root,
                Err(error) => return result.and_then(|_| Err(error.into())),
            };
            // Selection and proving must observe one committed state. All attempted
            // reads remain in the cost accumulator, including discarded proofs.
            // A failed attempt has no proof to discard; preserve its original fault.
            if root_before != root_after && result.is_ok() {
                continue;
            }
            return result;
        }
        Err(Error::Drive(DriveError::ProofGenerationRetriesExhausted))
    }
}

#[cfg(test)]
mod selection_hook {
    use super::Drive;
    use std::cell::RefCell;

    type Hook = Box<dyn FnMut(&Drive)>;
    thread_local! {
        static AFTER_SELECTION: RefCell<Option<Hook>> = RefCell::new(None);
    }

    pub(super) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            AFTER_SELECTION.with(|slot| slot.replace(None));
        }
    }

    pub(super) fn install(hook: impl FnMut(&Drive) + 'static) -> Guard {
        AFTER_SELECTION.with(|slot| {
            assert!(slot.borrow().is_none(), "selection hook cannot be nested");
            slot.replace(Some(Box::new(hook)));
        });
        Guard
    }

    pub(super) fn run(drive: &Drive) {
        let hook = AFTER_SELECTION.with(|slot| slot.take());
        if let Some(mut hook) = hook {
            hook(drive);
            AFTER_SELECTION.with(|slot| slot.replace(Some(hook)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn should_prove_a_single_identity_with_non_unique_key() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        let first_key_hash = identity
            .public_keys()
            .values()
            .find(|public_key| !public_key.key_type().is_unique_key_type())
            .expect("expected a unique key")
            .public_key_hash()
            .expect("expected to hash data");

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                first_key_hash,
                None,
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identity) = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            first_key_hash,
            None,
            platform_version,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity, Some(identity));
    }

    #[test]
    fn should_prove_a_single_identity_with_non_unique_key_when_two_have_same_key() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a random identity");

        let mut identity_2 = Identity::random_identity(3, Some(15), platform_version)
            .expect("expected a random identity");

        let mut rng = StdRng::seed_from_u64(506);

        let key = IdentityPublicKey::random_voting_key_with_rng(3, &mut rng, platform_version)
            .expect("expected key")
            .0;

        identity_1.add_public_key(key.clone());
        identity_2.add_public_key(key.clone());

        drive
            .add_new_identity(
                identity_1.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .add_new_identity(
                identity_2.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        let key_hash = key.public_key_hash().expect("expected key hash");

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                key_hash,
                None,
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identity) = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            key_hash,
            None,
            platform_version,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity, Some(identity_1.clone()));

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                key_hash,
                Some(identity_1.id().to_buffer()),
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identity) = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            key_hash,
            Some(identity_1.id().to_buffer()),
            platform_version,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity, Some(identity_2.clone()));

        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                key_hash,
                Some(identity_2.id().to_buffer()),
                None,
                platform_version,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identity) = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            key_hash,
            Some(identity_2.id().to_buffer()),
            platform_version,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity, None);
    }

    #[test]
    fn should_retry_when_an_earlier_nonunique_identity_is_committed_after_selection() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut rng = StdRng::seed_from_u64(506);
        let key = IdentityPublicKey::random_voting_key_with_rng(3, &mut rng, platform_version)
            .expect("shared voting key")
            .0;
        let hash = key.public_key_hash().expect("key hash");
        let mut original =
            Identity::random_identity(3, Some(101), platform_version).expect("original identity");
        original.set_id([0x10; 32].into());
        original.add_public_key(key.clone());
        drive
            .add_new_identity(
                original,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("store original first identity");
        let mut earlier = Identity::random_identity(3, Some(102), platform_version)
            .expect("new earlier identity");
        earlier.set_id([0x05; 32].into());
        earlier.add_public_key(key);
        let expected = earlier.clone();
        let mut inserted = false;
        let _hook = selection_hook::install(move |drive| {
            if !inserted {
                inserted = true;
                drive
                    .add_new_identity(
                        earlier.clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("commit an earlier identity after selection");
            }
        });
        let proof = drive
            .prove_full_identity_by_non_unique_public_key_hash(hash, None, None, platform_version)
            .expect("coherent merged producer");
        let (root, identity) = Drive::verify_full_identity_by_non_unique_public_key_hash(
            &proof,
            hash,
            None,
            platform_version,
            platform_version,
        )
        .expect("root drift retries the whole lookup rather than serving an incoherent selection");
        assert_eq!(identity, Some(expected));
        assert_eq!(
            root,
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("root")
        );
    }
    #[test]
    fn should_bound_retries_and_retain_every_attempt_read_cost() {
        use crate::drive::balances::balance_path_vec;
        use std::cell::Cell;
        use std::rc::Rc;
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let identity = Identity::random_identity(3, Some(14), platform_version).expect("identity");
        let hash = identity
            .public_keys()
            .values()
            .find(|key| !key.key_type().is_unique_key_type())
            .expect("non-unique key")
            .public_key_hash()
            .expect("key hash");
        let id = identity.id().to_buffer();
        drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("store identity");
        let mut one_attempt = vec![];
        drive
            .prove_full_identity_by_non_unique_public_key_hash_v0_operations(
                hash,
                None,
                None,
                &mut one_attempt,
                platform_version,
            )
            .expect("stable control");
        let attempts = Rc::new(Cell::new(0u64));
        let hook_attempts = Rc::clone(&attempts);
        let _hook = selection_hook::install(move |drive| {
            let next = hook_attempts.get() + 1;
            hook_attempts.set(next);
            drive
                .grove
                .insert(
                    balance_path_vec().as_slice(),
                    &id,
                    grovedb::Element::new_item(next.to_be_bytes().to_vec()),
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("commit balance after selection");
        });
        let mut all_attempts = vec![];
        let error = drive
            .prove_full_identity_by_non_unique_public_key_hash_v0_operations(
                hash,
                None,
                None,
                &mut all_attempts,
                platform_version,
            )
            .expect_err("persistent drift is transient, not a successful incoherent proof");
        assert!(
            !matches!(error, Error::Drive(DriveError::NotSupported(_))),
            "a commit race must not be classified as an unsupported operation: {error:?}"
        );
        assert!(matches!(
            error,
            Error::Drive(DriveError::ProofGenerationRetriesExhausted)
        ));
        assert_eq!(attempts.get(), 3);
        assert_eq!(all_attempts.len(), one_attempt.len() * 3);
        assert!(all_attempts
            .iter()
            .all(|op| matches!(op, LowLevelDriveOperation::CalculatedCostOperation(_))));
        let seeks = |ops: &[LowLevelDriveOperation]| {
            ops.iter()
                .map(|op| match op {
                    LowLevelDriveOperation::CalculatedCostOperation(cost) => cost.seek_count,
                    _ => 0,
                })
                .sum::<u32>()
        };
        assert!(seeks(&all_attempts) > seeks(&one_attempt));
    }

    #[test]
    fn should_preserve_raw_proving_error_without_root_drift() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let hash = [7; 20];
        let transaction = drive.grove.start_transaction();
        let query = Drive::identity_id_by_non_unique_public_key_hash_query(hash, None);
        let expected = drive
            .grove
            .get_proved_path_query(
                &query,
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect_err("raw backend does not support transaction proofs");
        let error = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                hash,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect_err("proof must retain raw backend fault");
        assert!(
            matches!(&error, Error::GroveDB(error) if matches!(error.as_ref(), grovedb::Error::NotSupported(_)))
        );
        assert_eq!(error.to_string(), Error::from(expected).to_string());
    }

    #[test]
    fn should_preserve_raw_proving_error_during_root_drift() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let hash = [7; 20];
        let transaction = drive.grove.start_transaction();
        let query = Drive::identity_id_by_non_unique_public_key_hash_query(hash, None);
        let expected = drive
            .grove
            .get_proved_path_query(
                &query,
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect_err("raw backend does not support transaction proofs");
        let attempts = Rc::new(Cell::new(0u64));
        let hook_attempts = Rc::clone(&attempts);
        let _hook = selection_hook::install(move |drive| {
            let next = hook_attempts.get() + 1;
            hook_attempts.set(next);
            drive
                .grove
                .insert(
                    &[] as &[&[u8]],
                    &[0xff],
                    grovedb::Element::new_item(next.to_be_bytes().to_vec()),
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("commit after transaction selection");
        });
        let error = drive
            .prove_full_identity_by_non_unique_public_key_hash(
                hash,
                None,
                Some(&transaction),
                platform_version,
            )
            .expect_err("proof must retain raw backend fault");
        assert!(
            matches!(&error, Error::GroveDB(error) if matches!(error.as_ref(), grovedb::Error::NotSupported(_)))
        );
        assert_eq!(error.to_string(), Error::from(expected).to_string());
        assert_eq!(
            attempts.get(),
            1,
            "a backend fault is not retried on root drift"
        );
    }
}
