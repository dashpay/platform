use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use crate::fee_pools::epochs::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::{Identity, IdentityPublicKey, KeyType};
use grovedb::TransactionArg;

pub fn create_test_identity(drive: &Drive, id: [u8; 32], transaction: TransactionArg) -> Identity {
    let identity_key = IdentityPublicKey {
        id: 1,
        key_type: KeyType::ECDSA_SECP256K1,
        data: vec![0, 1, 2, 3],
        purpose: dpp::identity::Purpose::AUTHENTICATION,
        security_level: dpp::identity::SecurityLevel::MASTER,
        read_only: false,
    };

    let identity = Identity {
        id: Identifier::new(id),
        revision: 1,
        balance: 0,
        protocol_version: 0,
        public_keys: vec![identity_key],
        asset_lock_proof: None,
        metadata: None,
    };

    drive
        .insert_identity(identity.clone(), true, StorageFlags::default(), transaction)
        .expect("should insert identity");

    identity
}

pub fn increment_in_epoch_each_proposers_block_count(
    drive: &Drive,
    epoch_tree: &Epoch,
    proposers: &Vec<[u8; 32]>,
    transaction: TransactionArg,
) {
    let mut batch = GroveDbOpBatch::new();

    for proposer_pro_tx_hash in proposers {
        let op = epoch_tree
            .increment_proposer_block_count_operation(
                &drive,
                &proposer_pro_tx_hash,
                None,
                transaction,
            )
            .expect("should increment proposer block count");
        batch.push(op);
    }

    drive
        .grove_apply_batch(batch, true, transaction)
        .expect("should apply batch");
}

pub fn create_test_masternode_identities_and_add_them_as_epoch_block_proposers(
    drive: &Drive,
    epoch: &Epoch,
    count: u16,
    transaction: TransactionArg,
) -> Vec<[u8; 32]> {
    let proposers = create_test_masternode_identities(drive, count, transaction);

    increment_in_epoch_each_proposers_block_count(drive, epoch, &proposers, transaction);

    proposers
}

pub fn create_test_masternode_identities(
    drive: &Drive,
    count: u16,
    transaction: TransactionArg,
) -> Vec<[u8; 32]> {
    let mut identity_ids: Vec<[u8; 32]> = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let proposer_pro_tx_hash: [u8; 32] = rand::random();

        create_test_identity(drive, proposer_pro_tx_hash, transaction);

        identity_ids.push(proposer_pro_tx_hash);
    }

    identity_ids
}
