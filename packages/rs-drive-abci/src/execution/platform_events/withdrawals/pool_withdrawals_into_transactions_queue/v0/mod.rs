use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions
    pub(super) fn pool_withdrawals_into_transactions_queue_v0(
        &self,
        block_info: &BlockInfo,
        last_committed_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Currently Core only supports using the first 2 quorums (out of 24 for mainnet).
        // For extra safety, we use only the first quorum because our quorum info based
        // on core chain locked height which is always late comparing with Core.
        let Some(position_of_current_quorum) =
            last_committed_platform_state.current_validator_set_position_in_list_by_most_recent()
        else {
            tracing::warn!("Current quorum not in current validator set, do not pool withdrawals");

            return Ok(());
        };
        if position_of_current_quorum != 0 {
            tracing::debug!(
                "Current quorum is not most recent, it is in position {}, do not pool withdrawals",
                position_of_current_quorum
            );

            return Ok(());
        }
        // Just use the v1 as to not duplicate code
        self.pool_withdrawals_into_transactions_queue_v1(block_info, transaction, platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::util::test_helpers::setup::{setup_document, setup_system_data_contract};
    use itertools::Itertools;

    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::system_data_contracts::{load_system_data_contract, withdrawals_contract};
    use dpp::version::PlatformVersion;
    use drive::config::DEFAULT_QUERY_LIMIT;

    #[test]
    fn test_pooling() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 96,
            epoch: Epoch::default(),
        };

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        let owner_id = Identifier::new([1u8; 32]);

        let document_1 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 1u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        setup_document(
            &platform.drive,
            &document_1,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let document_2 = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 2u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        setup_document(
            &platform.drive,
            &document_2,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let platform_state = platform.state.load();

        platform
            .pool_withdrawals_into_transactions_queue_v0(
                &block_info,
                &platform_state,
                Some(&transaction),
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");

        let updated_documents = platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");

        for (i, document) in updated_documents
            .into_iter()
            // Sort by index because updated_at is the same for all documents within batch
            .sorted_by(|a, b| {
                let a_index = a
                    .properties()
                    .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                    .expect("to get transactionIndex");
                let b_index = b
                    .properties()
                    .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                    .expect("to get transactionIndex");
                a_index.cmp(&b_index)
            })
            .enumerate()
        {
            assert_eq!(document.revision(), Some(2));

            let tx_index = document
                .properties()
                .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                .expect("to get transactionIndex");

            assert_eq!(tx_index, i as u64);
        }
    }
}
