// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Drive Identity Helpers.
//!
//! This module defines helper functions pertinent to identities in Drive.
//!

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::Drive;
use crate::fee_pools::epochs::Epoch;
use dpp::identifier::Identifier;
use dpp::identity::{Identity, IdentityPublicKey, KeyType};
use grovedb::TransactionArg;

/// Creates a test identity from an id and inserts it into Drive.
pub fn create_test_identity(drive: &Drive, id: [u8; 32], transaction: TransactionArg) -> Identity {
    let identity_key = IdentityPublicKey {
        id: 1,
        key_type: KeyType::ECDSA_SECP256K1,
        data: vec![0, 1, 2, 3],
        purpose: dpp::identity::Purpose::AUTHENTICATION,
        security_level: dpp::identity::SecurityLevel::MASTER,
        read_only: false,
        disabled_at: None,
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

/// Increments each proposer in the list given's block count by 1.
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

/// Creates test masternode identities and adds them as epoch block proposers.
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

/// Creates a list of test Masternode identities of size `count` with random data
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
