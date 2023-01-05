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
use drive::drive::batch::GroveDbOpBatch;
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
use rand::Rng;
use drive::contract::{Contract, CreateRandomDocument, DocumentType};
use drive::dpp::identifier::Identifier;
use drive::drive::{block_info, Drive};
use drive::drive::flags::StorageFlags;
use drive::drive::flags::StorageFlags::SingleEpoch;
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use drive::fee::result::FeeResult;

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
}

impl TestParams {
    fn execute_block_identity_operations(
        &mut self,
        drive: &Drive,
        block_info: &BlockInfo,
        transaction: &Transaction,
        rng: &mut StdRng,
    ) {
        let frequency = &self.strategy.identities_inserts;
        if frequency.check_hit(rng) {
            let count = frequency.events(rng);
            create_identities(drive, block_info, count, 5, Some(transaction),rng: &mut StdRng)
        }
    }

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

fn create_identities(
    drive: &Drive,
    block_info: &BlockInfo,
    count: u16,
    key_count: KeyID,
    transaction: TransactionArg,
    rng: &mut StdRng,
) -> Vec<Identity> {
    let identities = Identity::random_identities_with_rng(count, key_count, rng);
    for identity in identities.clone() {
        let balance = identity.balance;
        drive
            .add_new_identity(identity, block_info, true, transaction)
            .expect("expected to add an identity");
        drive.add_to_system_credits(balance, transaction).expect("added to system credits")
    }
    identities
}

fn run_block(platform: &Platform, proposer: [u8;32], block_info: &BlockInfo, test_params: &mut TestParams, rng: &mut StdRng) {

    let transaction = platform.drive.grove.start_transaction();
    // Processing block
    let block_begin_request = BlockBeginRequest {
        block_height: block_info.height,
        block_time_ms: block_info.time_ms,
        previous_block_time_ms,
        proposer_pro_tx_hash: proposers[block_height as usize % (proposers_count as usize)],
        validator_set_quorum_hash: Default::default(),
    };

    let block_begin_response = platform
        .block_begin(block_begin_request, Some(&transaction))
        .unwrap_or_else(|_| {
            panic!(
                "should begin process block #{} for day #{}",
                block_height, day
            )
        });

    test_params.execute_block(&platform.drive, block_info, &transaction, rng);

    let block_end_request = BlockEndRequest {
        fees: BlockFees {
            storage_fee: storage_fees_per_block,
            processing_fee: 1600,
            fee_refunds: CreditsPerEpoch::from_iter([(0, 100)]),
        },
    };

    let block_end_response = platform
        .block_end(block_end_request, Some(&transaction))
        .expect(
            format!(
                "should end process block #{} for day #{}",
                block_height, day
            )
                .as_str(),
        );

    let after_finalize_block_request = AfterFinalizeBlockRequest {
        updated_data_contract_ids: Vec::new(),
    };

    platform
        .after_finalize_block(after_finalize_block_request)
        .unwrap_or_else(|_| {
            panic!(
                "should begin process block #{} for day #{}",
                block_height, day
            )
        });
}

fn run_chain(platform: &Platform, days: u32, blocks_per_day: u32, block_interval_s: u32,
             mut test_params: TestParams, rng: &mut StdRng) {

    let mut quorum_members = test_params.quorum_members.clone();
    let genesis_time = Utc::now();
    let mut block_height = 0;
    // process blocks
    for day in 0..days {
        for block_num in 0..blocks_per_day {
            let block_time = if day == 0 && block_num == 0 {
                genesis_time
            } else {
                genesis_time
                    + Duration::days(day as i64)
                    + Duration::seconds(block_interval_s as i64 * block_num as i64)
            };
            let block_info = BlockInfo {
                time_ms: block_time.timestamp_millis() as u64,
                height: block_height,
                epoch: Default::default(),
            };

            let proposer = quorum_members.remove(0).unwrap();
            run_block(platform, proposer, &block_info, &mut test_params, rng);
            quorum_members.push_back(proposer);
            block_height += 1;
        }
    }
}

//todo redo
#[test]
fn play_chain() {
    let platform = setup_platform_raw(Some(PlatformConfig {
        drive_config: Default::default(),
        verify_sum_trees: false,
    }));
    let transaction = platform.drive.grove.start_transaction();

    // init chain
    let init_chain_request = InitChainRequest {};

    platform
        .init_chain(init_chain_request, Some(&transaction))
        .expect("should init chain");

    // Init withdrawal requests
    let withdrawals = (0..16)
        .map(|index: u64| (index.to_be_bytes().to_vec(), vec![index as u8; 32]))
        .collect();

    let mut batch = GroveDbOpBatch::new();

    platform
        .drive
        .add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawals);

    platform
        .drive
        .grove_apply_batch(batch, true, Some(&transaction))
        .expect("to apply batch");

    // setup the contract
    let contract = platform.create_mn_shares_contract(Some(&transaction));

    let genesis_time = Utc::now();

    let total_days = 29;

    let epoch_1_start_day = 18;

    let blocks_per_day = 50i64;

    let epoch_1_start_block = 13;

    let proposers_count = 50u16;

    let storage_fees_per_block = 42000;

    // and create masternode identities
    let proposers = create_test_masternode_identities(
        &platform.drive,
        proposers_count,
        Some(51),
        Some(&transaction),
    );

    create_test_masternode_share_identities_and_documents(
        &platform.drive,
        &contract,
        &proposers,
        Some(53),
        Some(&transaction),
    );

    let block_interval = 86400i64.div(blocks_per_day);

    let mut previous_block_time_ms: Option<u64> = None;
}
