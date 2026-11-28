use dpp::block::block_info::BlockInfo;
use metrics::gauge;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use crate::metrics::{
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE, GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE,
    GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL,
};
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions.
    ///
    /// Version 3 pools up to the same amount as version 2, the smaller of what the daily
    /// withdrawal limit and the Core-anchored limit have left, and differs in what it does
    /// with a queued withdrawal that does not fit it. Version 2 stops there, so nothing behind
    /// that withdrawal pools until it fits. That is safe while every withdrawal fits
    /// eventually, which the floor of the Core-anchored limit guaranteed up to protocol
    /// version 15 by being at least one maximal withdrawal. Protocol version 16 raises the
    /// largest withdrawal above that floor, which cannot follow it past Core's own: a
    /// withdrawal larger than the Core-anchored share of Core's credit pool fits only once the
    /// pool has grown, and would hold every withdrawal queued behind it until then.
    ///
    /// Version 3 passes over a withdrawal that does not fit and goes on to the ones behind it:
    ///
    /// * it reads the `withdrawal_pooling_candidates_per_block_limit` oldest queued
    ///   withdrawals, in the order version 2 reads them, where version 2 reads
    ///   `withdrawal_transactions_per_block_limit`;
    /// * it pools each one that fits what is left of the limit after the ones pooled before
    ///   it in the block, until `withdrawal_transactions_per_block_limit` are pooled;
    /// * a withdrawal passed over stays queued and is the first considered in the next block.
    ///
    /// The queue is therefore not served strictly in order: while the limits are short, a
    /// smaller withdrawal is pooled before an older, larger one, which waits until the limits
    /// have that much left when its turn comes.
    pub(super) fn pool_withdrawals_into_transactions_queue_v3(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Bring the Core blocks up to date first, every block whether or not anything is
        // queued: their credit pool balances feed the Core-anchored limit below, and a long
        // jump of the chain locked height is read over several blocks.
        self.scan_core_blocks_for_withdrawals(block_info, transaction, platform_version)?;

        let system_limits = &platform_version.system_limits;

        let candidates_limit = system_limits
            .withdrawal_pooling_candidates_per_block_limit
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "pool_withdrawals_into_transactions_queue v3 requires a pooling candidates limit",
            )))?;

        let documents = self.drive.fetch_oldest_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            candidates_limit,
            transaction,
            platform_version,
        )?;

        if documents.is_empty() {
            tracing::debug!(
                height = block_info.height,
                candidates_limit,
                "No queued withdrawal documents found to pool into transactions"
            );
            return Ok(());
        }

        let withdrawals_info = self.drive.calculate_current_withdrawal_limit(
            block_info,
            transaction,
            platform_version,
        )?;

        // Computed whenever withdrawals are queued, so its gauge stays current. Once the scan
        // has recorded the band and the chain locked height, it reads state only.
        let core_anchored_withdrawal_limit = self.calculate_core_anchored_withdrawal_limit(
            block_info,
            transaction,
            platform_version,
        )?;

        let current_withdrawal_limit = withdrawals_info
            .available()
            .min(core_anchored_withdrawal_limit);

        tracing::trace!(
            ?withdrawals_info,
            core_anchored_withdrawal_limit,
            documents_count = documents.len(),
            "Calculated withdrawal limits"
        );

        // Store prometheus metrics: what may be pooled, and the Core side on its own.
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_AVAILABLE).set(current_withdrawal_limit as f64);
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_TOTAL).set(withdrawals_info.daily_maximum as f64);
        gauge!(GAUGE_CREDIT_WITHDRAWAL_LIMIT_CORE_AVAILABLE)
            .set(core_anchored_withdrawal_limit as f64);

        let pooled_per_block_limit =
            usize::from(system_limits.withdrawal_transactions_per_block_limit);

        let mut total_withdrawal_amount = 0u64;
        let mut passed_over = 0usize;

        let mut documents_to_process = vec![];
        for document in documents {
            if documents_to_process.len() >= pooled_per_block_limit {
                break;
            }

            let amount: u64 = document
                .properties()
                .get_integer(withdrawal::properties::AMOUNT)?;

            // A total past `u64` is past every limit, so it does not fit either.
            let Some(potential_total_withdrawal_amount) = total_withdrawal_amount
                .checked_add(amount)
                .filter(|total| *total <= current_withdrawal_limit)
            else {
                passed_over += 1;
                continue;
            };

            total_withdrawal_amount = potential_total_withdrawal_amount;

            documents_to_process.push(document);
        }

        if passed_over > 0 {
            tracing::debug!(
                passed_over,
                current_withdrawal_limit,
                "Passed over queued withdrawals that do not fit the withdrawal limits"
            );
        }

        if documents_to_process.is_empty() {
            tracing::debug!(
                block_info = %block_info,
                "No withdrawal documents to process"
            );
            return Ok(());
        }

        let start_transaction_index = self
            .drive
            .fetch_next_withdrawal_transaction_index(transaction, platform_version)?;

        let (withdrawal_transactions, total_amount) = self
            .build_untied_withdrawal_transactions_from_documents(
                &mut documents_to_process,
                start_transaction_index,
                block_info,
                transaction,
                platform_version,
            )?;

        let withdrawal_transactions_count = withdrawal_transactions.len();

        let mut drive_operations = vec![];

        self.drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                withdrawal_transactions,
                total_amount,
                &mut drive_operations,
                platform_version,
            )?;

        let end_transaction_index = start_transaction_index + withdrawal_transactions_count as u64;

        self.drive
            .add_update_next_withdrawal_transaction_index_operation(
                end_transaction_index,
                &mut drive_operations,
                platform_version,
            )?;

        tracing::debug!(
            "Pooled {} withdrawal documents into {} transactions with indices from {} to {}",
            documents_to_process.len(),
            withdrawal_transactions_count,
            start_transaction_index,
            end_transaction_index,
        );

        let withdrawals_contract = self
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)?;

        self.drive.add_update_multiple_documents_operations(
            &documents_to_process,
            &withdrawals_contract,
            withdrawals_contract
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            transaction,
            platform_version,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::balances::credits::CREDITS_PER_DUFF;
    use dpp::block::epoch::Epoch;
    use dpp::dash_to_credits;
    use dpp::data_contracts::SystemDataContract;
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};

    const DUFFS_PER_DASH: u64 = 100_000_000;
    // `dash_to_credits!` reads a literal only.
    const CREDITS_PER_DASH: Credits = DUFFS_PER_DASH * CREDITS_PER_DUFF;

    /// What `pool` left in each state, in Dash.
    #[derive(Debug, PartialEq, Eq)]
    struct PoolingOutcome {
        /// The pooled withdrawals, in the order of their transaction indices.
        pooled: Vec<u64>,
        /// The withdrawals still queued, in queue order.
        queued: Vec<u64>,
    }

    /// Queues one withdrawal per entry of `amounts_in_dash`, in that order, and pools them
    /// once through the dispatcher at `platform_version`, against a Core whose credit pool
    /// holds `pool_in_dash` at every height. `credit_inflow_in_dash` is recorded as deposited
    /// in the block: without a day of total credits history the daily maximum is the flat
    /// 2000 Dash plus it.
    ///
    /// The queue is read by Core fee rate before age, so each withdrawal gets a higher rate
    /// than the one before it to fix the order.
    fn pool(
        platform_version: &PlatformVersion,
        amounts_in_dash: &[u64],
        pool_in_dash: u64,
        credit_inflow_in_dash: u64,
    ) -> PoolingOutcome {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_credit_pool_balance()
            .returning(move |_| Ok(pool_in_dash * DUFFS_PER_DASH));
        core_rpc
            .expect_get_coinbase_payload_version()
            .returning(|_| Ok(Some(3)));
        platform.core_rpc = core_rpc;

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 10_000,
            epoch: Epoch::default(),
        };

        if credit_inflow_in_dash > 0 {
            platform
                .drive
                .record_credit_inflow(
                    credit_inflow_in_dash * CREDITS_PER_DASH,
                    &block_info,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record the credit inflow");
        }

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));
        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        for (position, amount_in_dash) in amounts_in_dash.iter().enumerate() {
            let document = get_withdrawal_document_fixture(
                &data_contract,
                Identifier::new([1u8; 32]),
                platform_value!({
                    "amount": amount_in_dash * CREDITS_PER_DASH,
                    "coreFeePerByte": position as u32 + 1,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": position as u64,
                }),
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");
            setup_document(
                &platform.drive,
                &document,
                &data_contract,
                document_type,
                Some(&transaction),
            );
        }

        let platform_state = platform.state.load();
        platform
            .pool_withdrawals_into_transactions_queue(
                &block_info,
                &platform_state,
                Some(&transaction),
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");

        let amounts_in_dash_with_status = |status: withdrawals_contract::WithdrawalStatus| {
            platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    status.into(),
                    DEFAULT_QUERY_LIMIT,
                    Some(&transaction),
                    platform_version,
                )
                .expect("to fetch withdrawal documents")
                .into_iter()
                .map(|document| {
                    let amount: Credits = document
                        .properties()
                        .get_integer(withdrawal::properties::AMOUNT)
                        .expect("expected the amount");
                    let transaction_index: u64 = document
                        .properties()
                        .get_integer(withdrawal::properties::TRANSACTION_INDEX)
                        .expect("expected the transaction index");
                    (transaction_index, amount / CREDITS_PER_DASH)
                })
                .collect::<Vec<_>>()
        };

        let mut pooled =
            amounts_in_dash_with_status(withdrawals_contract::WithdrawalStatus::POOLED);
        pooled.sort();

        PoolingOutcome {
            pooled: pooled.into_iter().map(|(_, amount)| amount).collect(),
            queued: amounts_in_dash_with_status(withdrawals_contract::WithdrawalStatus::QUEUED)
                .into_iter()
                .map(|(_, amount)| amount)
                .collect(),
        }
    }

    /// A 10,000 Dash credit pool gives up 1500 Dash, the floor of the Core-anchored limit: a
    /// maximal withdrawal never fits it. Version 3 pools the withdrawals behind it; version 2,
    /// which protocol version 15 selects, pools nothing while it is the oldest.
    #[test]
    fn should_pool_the_withdrawals_behind_one_that_does_not_fit_only_from_protocol_version_16() {
        let version_15 = PlatformVersion::get(15).expect("expected protocol version 15");

        assert_eq!(
            pool(PlatformVersion::latest(), &[4000, 100, 200], 10_000, 0),
            PoolingOutcome {
                pooled: vec![100, 200],
                queued: vec![4000],
            }
        );
        assert_eq!(
            pool(version_15, &[4000, 100, 200], 10_000, 0),
            PoolingOutcome {
                pooled: vec![],
                queued: vec![4000, 100, 200],
            }
        );
    }

    /// What is pooled earlier in the block counts against the ones after it: of 1500 Dash,
    /// 1000 leave 500, which the 600 and, after the 400, the 200 do not fit.
    #[test]
    fn should_count_what_it_pooled_against_the_withdrawals_after_one_it_passed_over() {
        assert_eq!(
            pool(PlatformVersion::latest(), &[1000, 600, 400, 200], 10_000, 0),
            PoolingOutcome {
                pooled: vec![1000, 400],
                queued: vec![600, 200],
            }
        );
    }

    /// A maximal withdrawal is pooled where both limits admit it: 15% of a 30,000 Dash
    /// credit pool is 4500 Dash, and the deposit that funded it raises the daily maximum by
    /// as much. At 20,000 Dash the pool gives up 3000 Dash, and the withdrawal waits while the
    /// one behind it pools.
    #[test]
    fn should_pool_a_maximal_withdrawal_once_cores_credit_pool_admits_it() {
        let platform_version = PlatformVersion::latest();
        assert_eq!(
            platform_version.system_limits.max_withdrawal_amount,
            dash_to_credits!(4000)
        );

        assert_eq!(
            pool(platform_version, &[4000, 100], 30_000, 4000),
            PoolingOutcome {
                pooled: vec![4000, 100],
                queued: vec![],
            }
        );
        assert_eq!(
            pool(platform_version, &[4000, 100], 20_000, 4000),
            PoolingOutcome {
                pooled: vec![100],
                queued: vec![4000],
            }
        );
    }

    /// Passing over withdrawals does not raise how many a block pools.
    #[test]
    fn should_pool_no_more_withdrawals_per_block_than_before() {
        assert_eq!(
            pool(
                PlatformVersion::latest(),
                &[1, 2000, 2, 3, 4, 5, 6],
                10_000,
                0
            ),
            PoolingOutcome {
                pooled: vec![1, 2, 3, 4],
                queued: vec![2000, 5, 6],
            }
        );
    }

    /// Only the oldest queued withdrawals are considered, as many as the candidates limit: a
    /// withdrawal behind that many that do not fit is not reached.
    #[test]
    fn should_consider_no_more_queued_withdrawals_than_the_candidates_limit() {
        let platform_version = PlatformVersion::latest();
        let candidates_limit = platform_version
            .system_limits
            .withdrawal_pooling_candidates_per_block_limit
            .expect("expected a pooling candidates limit");

        let mut amounts = vec![2000; usize::from(candidates_limit) - 1];
        amounts.push(1);
        amounts.push(2);
        let outcome = pool(platform_version, &amounts, 10_000, 0);
        assert_eq!(outcome.pooled, vec![1]);
        assert_eq!(outcome.queued.len(), usize::from(candidates_limit));
    }
}
