//! Drive Identity Helpers.
//!
//! This module defines helper functions pertinent to identities in Drive.
//!

use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee_pools::epochs::operations_factory::EpochOperations;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;

/// Creates a test identity from an id and inserts it into Drive.
pub fn create_test_identity(
    drive: &Drive,
    id: [u8; 32],
    seed: Option<u64>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Identity, Error> {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };

    create_test_identity_with_rng(drive, id, &mut rng, transaction, platform_version)
}

/// Creates multiple test identities with random generator and inserts them into Drive.
///
/// # Arguments
///
/// * `drive` - A reference to the Drive.
/// * `ids` - An IntoIterator of [u8; 32] representing the ids for the test identities to create.
/// * `rng` - A mutable reference to the random number generator.
/// * `transaction` - A transaction argument to interact with the underlying storage.
///
/// # Returns
///
/// * `Vec<Identity>` - Returns a vector of created test identities.
pub fn create_test_identities_with_rng<I>(
    drive: &Drive,
    ids: I,
    rng: &mut StdRng,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Vec<Identity>, Error>
where
    I: IntoIterator<Item = [u8; 32]>,
{
    let ids_iter = ids.into_iter();
    let mut identities = Vec::with_capacity(ids_iter.size_hint().0);

    for id in ids_iter {
        let identity =
            create_test_identity_with_rng(drive, id, rng, transaction, platform_version)?;
        identities.push(identity);
    }

    Ok(identities)
}

/// Creates a test identity from an id with random generator and inserts it into Drive.
pub fn create_test_identity_with_rng(
    drive: &Drive,
    id: [u8; 32],
    rng: &mut StdRng,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Identity, Error> {
    let (identity_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
        1,
        rng,
        platform_version,
    )?;

    let mut public_keys = BTreeMap::new();

    public_keys.insert(identity_key.id(), identity_key);

    let identity: Identity = IdentityV0 {
        id: Identifier::new(id),
        revision: 0,
        balance: 0,
        public_keys,
    }
    .into();

    drive
        .add_new_identity(
            identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            transaction,
            platform_version,
        )
        .expect("should insert identity");

    Ok(identity)
}

/// Increments each proposer in the list given's block count by 1.
pub fn increment_in_epoch_each_proposers_block_count(
    drive: &Drive,
    epoch_tree: &Epoch,
    proposers: &Vec<[u8; 32]>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) {
    let mut batch = GroveDbOpBatch::new();

    for proposer_pro_tx_hash in proposers {
        let op = epoch_tree
            .increment_proposer_block_count_operation(
                drive,
                proposer_pro_tx_hash,
                None,
                transaction,
                platform_version,
            )
            .expect("should increment proposer block count");
        batch.push(op);
    }

    drive
        .grove_apply_batch(batch, true, transaction, &platform_version.drive)
        .expect("should apply batch");
}

/// Creates test masternode identities and adds them as epoch block proposers.
pub fn create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
    drive: &Drive,
    epoch: &Epoch,
    count: u16,
    seed: Option<u64>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Vec<[u8; 32]> {
    let proposers =
        create_test_masternode_identities(drive, count, seed, transaction, platform_version);

    increment_in_epoch_each_proposers_block_count(
        drive,
        epoch,
        &proposers,
        transaction,
        platform_version,
    );

    proposers
}

/// Creates a list of test Masternode identities of size `count` with random data
pub fn create_test_masternode_identities(
    drive: &Drive,
    count: u16,
    seed: Option<u64>,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Vec<[u8; 32]> {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };
    create_test_masternode_identities_with_rng(
        drive,
        count,
        &mut rng,
        transaction,
        platform_version,
    )
}

/// Creates a list of test Masternode identities of size `count` with random data
pub fn create_test_masternode_identities_with_rng(
    drive: &Drive,
    count: u16,
    rng: &mut StdRng,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Vec<[u8; 32]> {
    let mut identity_ids: Vec<[u8; 32]> = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let proposer_pro_tx_hash = rng.gen::<[u8; 32]>();
        create_test_identity_with_rng(
            drive,
            proposer_pro_tx_hash,
            rng,
            transaction,
            platform_version,
        )
        .expect("expected to get a platform version");

        identity_ids.push(proposer_pro_tx_hash);
    }

    identity_ids
}

/// Creates a list of test Masternode identities of size `count` with random data
pub fn generate_pro_tx_hashes(count: u16, rng: &mut StdRng) -> Vec<[u8; 32]> {
    let mut identity_ids: Vec<[u8; 32]> = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let proposer_pro_tx_hash = rng.gen::<[u8; 32]>();
        identity_ids.push(proposer_pro_tx_hash);
    }

    identity_ids
}
