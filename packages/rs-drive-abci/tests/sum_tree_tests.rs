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

//! Execution Tests
//!

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use chrono::{Duration, Utc};
use drive::common::helpers::identities::create_test_masternode_identities;
use drive::dpp::identity::{Identity, KeyID};
use drive::drive::batch::{DriveOperationType, GroveDbOpBatch, IdentityOperationType, SystemOperationType};
use drive::drive::block_info::BlockInfo;
use drive::fee::epoch::CreditsPerEpoch;
use drive::grovedb::{Transaction, TransactionArg};
use drive_abci::abci::handlers::TenderdashAbci;
use drive_abci::abci::messages::{
    AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees, InitChainRequest,
};
use drive_abci::common::helpers::fee_pools::create_test_masternode_share_identities_and_documents;
use drive_abci::common::helpers::setup::setup_platform_raw;
use drive_abci::config::PlatformConfig;
use drive_abci::platform::Platform;
use rand::rngs::StdRng;
use rust_decimal::prelude::ToPrimitive;
use std::ops::{Div, Range};
use base64::Config;
use rand::{Rng, SeedableRng};
use drive::contract::{Contract, CreateRandomDocument, DocumentType};
use drive::dpp::identifier::Identifier;
use drive::dpp::identity;
use drive::drive::{block_info, Drive};
use drive::drive::flags::StorageFlags;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use drive::fee::result::FeeResult;
use drive_abci::execution::engine::ExecutionEvent;

#[derive(Clone, Debug)]
pub struct Frequency {
    pub times_per_block_range: Range<u16>, //insertion count when block is chosen
    pub chance_per_block: Option<f64>,     //chance of insertion if set
}

impl Frequency {
    fn check_hit(&self, rng: &mut StdRng) -> bool {
        match self.chance_per_block {
            None => true,
            Some(chance) => rng.gen_bool(chance),
        }
    }

    fn events(&self, rng: &mut StdRng) -> u16 {
        rng.gen_range(self.times_per_block_range.clone())
    }
}

#[derive(Clone, Debug)]
pub struct DocumentOp {
    pub contract: Contract,
    pub document_type: DocumentType,
}

pub type ProTxHash = [u8;32];

#[derive(Clone, Debug)]
struct Strategy {
    operations: Vec<(DocumentOp, Frequency)>,
    identities_inserts: Frequency,
}

#[derive(Clone, Debug)]
struct TestParams {
    strategy: Strategy,
    quorum_members: VecDeque<ProTxHash>,
    current_identities: BTreeMap<Identifier, Identity>
}

impl Strategy {
    fn add_strategy_contracts_into_drive(&mut self, drive: &Drive) {
        for (op, _) in &self.operations {
            let serialize = op.contract.to_cbor().expect("expected to serialize");
            drive.apply_contract(&op.contract, serialize, BlockInfo::default(), true, Some(&SingleEpoch(0)), None).expect("expected to be able to add contract");
        }
    }
    fn identity_operations_for_block(&self, rng: &mut StdRng) -> Vec<(Identity, Vec<DriveOperationType>)> {
        let frequency = &self.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            create_identities_operations(count, 5, rng)
        } else {
            vec![]
        }
    }

    fn document_operations_for_block(
        &mut self,
        cuurent_identities: &Vec<Identity>,
        rng: &mut StdRng,
    ) {
        for (op, frequency) in &self.operations {
            if frequency.check_hit(rng) {
                let count = rng.gen_range(frequency.times_per_block_range.clone());
                let documents = op.document_type.random_documents(count as u32, None);
                for document in &documents {
                    let i = rng.gen();
                    let identity = self.current_identities.values().get(i);
                    let serialization = document
                        .serialize(&op.document_type)
                        .expect("expected to serialize document");

                    let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(identity.id));

                    let estimated_document_fee_result = drive
                        .add_document_for_contract(
                            DocumentAndContractInfo {
                                owned_document_info: OwnedDocumentInfo {
                                    document_info: DocumentRefAndSerialization((
                                        document,
                                        serialization.as_slice(),
                                        storage_flags.as_ref(),
                                    )),
                                    owner_id: None
                                },
                                contract: &op.contract,
                                document_type: &op.document_type,
                            },
                            false,
                            block_info.clone(),
                            false,
                            Some(transaction),
                        )
                        .expect("expected to add document");

                    // does the identity have enough balance?

                    let balance = drive.fetch_identity_balance(identity.id, true, Some(transaction)).expect("expected to fetch identity balance").expect("expected to get balance");

                    if balance >= estimated_document_fee_result.total_fee() {
                        // then do the real operation
                        let fee_result = drive
                            .add_document_for_contract(
                                DocumentAndContractInfo {
                                    owned_document_info: OwnedDocumentInfo {
                                        document_info: DocumentRefAndSerialization((
                                            document,
                                            serialization.as_slice(),
                                            storage_flags.as_ref(),
                                        )),
                                        owner_id: None
                                    },
                                    contract: &op.contract,
                                    document_type: &op.document_type,
                                },
                                false,
                                block_info.clone(),
                                false,
                                Some(transaction),
                            )
                            .expect("expected to add document");

                        drive.remove_from_identity_balance(identity.id, fee_result.required_removed_balance(), fee_result.desired_removed_balance(), block_info, true, Some(transaction)).expect("expected to pay for operation");
                    }
                }
            }
        }

    fn state_transitions_for_block_with_new_identities(&self, rng: &mut StdRng) -> (Vec<ExecutionEvent>, Vec<Identity>) {
        let (identities, operations) : (Vec<Identity>, Vec<Vec<DriveOperationType>>) = self.identity_operations_for_block(rng).into_iter().unzip();
        let execution_events = operations.into_iter().map(|operation| ExecutionEvent::new_identity_insertion(operation)).collect();
        (execution_events, identities)
    }
}

impl TestParams {

    fn execute_block_document_operations(
        &mut self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: &Transaction,
        rng: &mut StdRng,
    ) {
        for (op, frequency) in &self.strategy.operations {
            if frequency.check_hit(rng) {
                let count = rng.gen_range(frequency.times_per_block_range.clone());
                let documents = op.document_type.random_documents(count as u32, None);
                for document in &documents {
                    let i = rng.gen();
                    let identity = self.current_identities.values().get(i);
                    let serialization = document
                        .serialize(&op.document_type)
                        .expect("expected to serialize document");

                    let storage_flags = StorageFlags::new_single_epoch(block_info.epoch.index, Some(identity.id));

                    let estimated_document_fee_result = drive
                        .add_document_for_contract(
                            DocumentAndContractInfo {
                                owned_document_info: OwnedDocumentInfo {
                                    document_info: DocumentRefAndSerialization((
                                        document,
                                        serialization.as_slice(),
                                        storage_flags.as_ref(),
                                    )),
                                    owner_id: None
                                },
                                contract: &op.contract,
                                document_type: &op.document_type,
                            },
                            false,
                            block_info.clone(),
                            false,
                            Some(transaction),
                        )
                        .expect("expected to add document");

                    // does the identity have enough balance?

                    let balance = drive.fetch_identity_balance(identity.id, true, Some(transaction)).expect("expected to fetch identity balance").expect("expected to get balance");

                    if balance >= estimated_document_fee_result.total_fee() {
                        // then do the real operation
                        let fee_result = drive
                            .add_document_for_contract(
                                DocumentAndContractInfo {
                                    owned_document_info: OwnedDocumentInfo {
                                        document_info: DocumentRefAndSerialization((
                                            document,
                                            serialization.as_slice(),
                                            storage_flags.as_ref(),
                                        )),
                                        owner_id: None
                                    },
                                    contract: &op.contract,
                                    document_type: &op.document_type,
                                },
                                false,
                                block_info.clone(),
                                false,
                                Some(transaction),
                            )
                            .expect("expected to add document");

                        drive.remove_from_identity_balance(identity.id, fee_result.required_removed_balance(), fee_result.desired_removed_balance(), block_info, true, Some(transaction)).expect("expected to pay for operation");
                    }
                }
            }
        }
    }

    fn execute_block(
        &mut self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: &Transaction,
        rng: &mut StdRng,
    ) {
        self.execute_block_identity_operations(drive, block_info, transaction, rng);
        self.execute_block_document_operations(drive, block_info, transaction, rng);
    }
}

fn create_identities_operations(
    count: u16,
    key_count: KeyID,
    rng: &mut StdRng,
) -> Vec<(Identity, Vec<DriveOperationType>)> {
    let identities = Identity::random_identities_with_rng(count, key_count, rng);
    identities.into_iter().map(|identity| {
        let insert_op = DriveOperationType::IdentityOperation(IdentityOperationType::AddNewIdentity { identity: identity.clone() });
        let system_credits_op = DriveOperationType::SystemOperation(SystemOperationType::AddToSystemCredits { amount: identity.balance});
        let ops = vec![insert_op,system_credits_op];
        (identity.clone(), ops)
    }).collect()
}


// fn run_chain(platform: &Platform, days: u32, blocks_per_day: u32, block_interval_s: u32,
//              mut test_params: TestParams, rng: &mut StdRng) {
//
//     let mut quorum_members = test_params.quorum_members.clone();
//     let genesis_time = Utc::now();
//     let mut block_height = 0;
//     // process blocks
//     for day in 0..days {
//         for block_num in 0..blocks_per_day {
//             let block_time = if day == 0 && block_num == 0 {
//                 genesis_time
//             } else {
//                 genesis_time
//                     + Duration::days(day as i64)
//                     + Duration::seconds(block_interval_s as i64 * block_num as i64)
//             };
//             let block_info = BlockInfo {
//                 time_ms: block_time.timestamp_millis() as u64,
//                 height: block_height,
//                 epoch: Default::default(),
//             };
//
//             let proposer = quorum_members.remove(0).unwrap();
//             run_block(platform, proposer, &block_info, &mut test_params, rng);
//             quorum_members.push_back(proposer);
//             block_height += 1;
//         }
//     }
// }


fn run_chain_for_strategy(block_count: u64, block_spacing_ms: u64,  strategy: Strategy, config: PlatformConfig, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut platform = setup_platform_raw(Some(config));
    let mut current_time_ms = 0;
    for block_height in 0..block_count {
        let block_info = BlockInfo {
            time_ms: current_time_ms,
            height: block_height,
            epoch: Default::default(),
        };
        let state_transitions = strategy.get_state_transitions(&mut rng);

        platform.execute_block(proposer, &block_info, state_transitions).expect("expected to execute a block");
        current_time_ms += block_spacing_ms;
    }

}

#[test]
fn run_chain_nothing_happening() {
    let strategy = Strategy {
        operations: vec![],
        identities_inserts: Frequency { times_per_block_range: Default::default(), chance_per_block: None },
    };
    let config = PlatformConfig {
        drive_config: Default::default(),
        verify_sum_trees: true,
    };
    run_chain_for_strategy(10000, 3000, strategy, config, 15);
}