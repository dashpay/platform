use crate::drive::tokens::paths::{
    token_root_perpetual_distributions_path, token_root_pre_programmed_distributions_path,
};
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::Transaction;

impl Drive {
    /// Creates the perpetual and pre-programmed distribution storage of every token in state
    /// that is configured with such a distribution but has no storage for it.
    ///
    /// Runs once, on the first block of protocol version 14. Before that version a contract
    /// update created only the balance, identity info, status, contract info and supply entries
    /// of a token it added, never the distribution storage the contract insert creates, so every
    /// claim on such a token failed as an internal error. From version 14 the update creates the
    /// storage itself (`update_contract` v2), but only for the tokens it adds, so the tokens
    /// added before it need theirs created here.
    ///
    /// A token whose storage exists is left untouched, so the walk is safe to repeat. A
    /// pre-programmed distribution that can not be stored at all (see
    /// [`pre_programmed_distribution_is_storable`]) is skipped: an error here would halt the
    /// chain on the upgrade block over a distribution nobody could ever have claimed.
    ///
    /// Returns the number of tokens that received storage.
    pub fn add_missing_token_distribution_storage_to_all_contracts(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        let mut start_at = None;
        let mut repaired_token_count = 0usize;

        loop {
            let page =
                self.fetch_contract_ids(start_at, u16::MAX, Some(transaction), platform_version)?;

            for contract_id in &page {
                repaired_token_count += self.add_missing_token_distribution_storage_to_contract(
                    *contract_id,
                    block_info,
                    transaction,
                    platform_version,
                )?;
            }

            match page.last() {
                Some(last_id) if page.len() == u16::MAX as usize => {
                    start_at = Some((*last_id, false));
                }
                _ => break,
            }
        }

        tracing::info!(
            repaired_token_count,
            "created the missing distribution storage of tokens added by a contract update"
        );

        Ok(repaired_token_count)
    }

    fn add_missing_token_distribution_storage_to_contract(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        let fetch_info = self
            .fetch_contract_and_add_operations(
                contract_id,
                None,
                Some(transaction),
                &mut vec![],
                platform_version,
            )?
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} is listed under the contracts root but can not be fetched",
                    hex::encode(contract_id)
                )))
            })?;
        let contract = &fetch_info.contract;

        let mut repaired_token_count = 0usize;

        for (token_pos, configuration) in contract.tokens() {
            let token_id = contract
                .token_id(*token_pos)
                .ok_or_else(|| {
                    Error::DataContract(DataContractError::CorruptedDataContract(format!(
                        "data contract has a token at position {}, but it can not be found",
                        token_pos
                    )))
                })?
                .to_buffer();

            let added_perpetual = self.add_missing_perpetual_distribution_storage(
                token_id,
                configuration,
                transaction,
                platform_version,
            )?;

            let added_pre_programmed = self.add_missing_pre_programmed_distribution_storage(
                contract,
                token_id,
                configuration,
                block_info,
                transaction,
                platform_version,
            )?;

            if added_perpetual || added_pre_programmed {
                repaired_token_count += 1;
            }
        }

        Ok(repaired_token_count)
    }

    fn add_missing_perpetual_distribution_storage(
        &self,
        token_id: [u8; 32],
        configuration: &TokenConfiguration,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(perpetual_distribution) =
            configuration.distribution_rules().perpetual_distribution()
        else {
            return Ok(false);
        };

        let has_storage = self.grove_has_raw(
            (&token_root_perpetual_distributions_path()).into(),
            &token_id,
            DirectQueryType::StatefulDirectQuery,
            Some(transaction),
            &mut vec![],
            &platform_version.drive,
        )?;
        if has_storage {
            return Ok(false);
        }

        // One batch per token and kind: the storage helpers look for an existing tree in
        // state only, never among the operations gathered so far.
        let mut batch_operations = vec![];
        self.add_perpetual_distribution(
            token_id,
            perpetual_distribution,
            &mut None,
            &mut batch_operations,
            Some(transaction),
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            Some(transaction),
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(true)
    }

    fn add_missing_pre_programmed_distribution_storage(
        &self,
        contract: &DataContract,
        token_id: [u8; 32],
        configuration: &TokenConfiguration,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(pre_programmed_distribution) = configuration
            .distribution_rules()
            .pre_programmed_distribution()
        else {
            return Ok(false);
        };

        let has_storage = self.grove_has_raw(
            (&token_root_pre_programmed_distributions_path()).into(),
            &token_id,
            DirectQueryType::StatefulDirectQuery,
            Some(transaction),
            &mut vec![],
            &platform_version.drive,
        )?;
        if has_storage {
            return Ok(false);
        }

        if !pre_programmed_distribution_is_storable(pre_programmed_distribution) {
            tracing::warn!(
                contract_id = %contract.id(),
                token_id = hex::encode(token_id),
                "skipped a pre-programmed distribution whose amounts do not fit a sum tree"
            );
            return Ok(false);
        }

        let mut batch_operations = vec![];
        self.add_pre_programmed_distributions(
            token_id,
            contract.owner_id().to_buffer(),
            pre_programmed_distribution,
            block_info,
            &mut None,
            &mut batch_operations,
            Some(transaction),
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            Some(transaction),
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(true)
    }
}

/// Whether the storage of `distribution` can be written: every release is a sum tree of its
/// recipients' amounts, so each amount and each release's total has to fit an `i64`.
///
/// No validation bounds these amounts. The contract insert rejects a distribution that fails
/// this as an internal error, but before protocol version 14 a contract update never wrote the
/// storage and so admitted it.
fn pre_programmed_distribution_is_storable(distribution: &TokenPreProgrammedDistribution) -> bool {
    distribution.distributions().values().all(|release| {
        release
            .values()
            .try_fold(0 as TokenAmount, |total, amount| total.checked_add(*amount))
            .is_some_and(|total| total <= i64::MAX as TokenAmount)
    })
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::paths::token_root_pre_programmed_distributions_path;
    use crate::drive::Drive;
    use crate::error::Error;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::balances::credits::TokenAmount;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::prelude::Identifier;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const RECIPIENT: [u8; 32] = [7; 32];

    fn upgrade_block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 5000,
            height: 500,
            core_height: 50,
            epoch: Default::default(),
        }
    }

    /// A token paying `RECIPIENT` 50 tokens every 10 blocks and, once, `amount` tokens at
    /// time 100.
    fn token_with_both_distributions(amount: TokenAmount) -> TokenConfiguration {
        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        let recipient = Identifier::from(RECIPIENT);
        configuration
            .distribution_rules_mut()
            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                TokenPerpetualDistributionV0 {
                    distribution_type: RewardDistributionType::BlockBasedDistribution {
                        interval: 10,
                        function: DistributionFunction::FixedAmount { amount: 50 },
                    },
                    distribution_recipient: TokenDistributionRecipient::Identity(recipient),
                },
            )));
        configuration
            .distribution_rules_mut()
            .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                TokenPreProgrammedDistributionV0 {
                    distributions: BTreeMap::from([(100, BTreeMap::from([(recipient, amount)]))]),
                },
            )));
        configuration
    }

    /// Under protocol version 13, registers a contract without tokens and adds `token` at
    /// position 0 through a contract update, which leaves it without distribution storage.
    /// Returns the token id.
    fn add_token_by_update_before_the_upgrade(
        drive: &Drive,
        contract_seed: u8,
        token: TokenConfiguration,
    ) -> [u8; 32] {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_id([contract_seed; 32].into());
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to insert the contract without tokens");

        contract.set_tokens(BTreeMap::from([(0, token)]));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected the update adding the token to succeed");

        contract
            .token_id(0)
            .expect("expected the token added at position 0")
            .to_buffer()
    }

    fn run_backfill(drive: &Drive) -> usize {
        let transaction = drive.grove.start_transaction();
        let repaired_token_count = drive
            .add_missing_token_distribution_storage_to_all_contracts(
                &upgrade_block_info(),
                &transaction,
                PlatformVersion::latest(),
            )
            .expect("expected the backfill to succeed");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        repaired_token_count
    }

    fn root_hash(drive: &Drive) -> [u8; 32] {
        drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    }

    /// Writes what a perpetual claim at block 40 and a claim of the release at time 100 write.
    fn record_both_claims(drive: &Drive, token_id: [u8; 32]) -> Result<(), Error> {
        let platform_version = PlatformVersion::latest();
        let mut operations = drive.mark_perpetual_release_as_distributed_operations(
            token_id,
            RECIPIENT,
            RewardDistributionMoment::BlockBasedMoment(40),
            &mut None,
            platform_version,
        )?;
        operations.extend(drive.mark_pre_programmed_release_as_distributed_operations(
            token_id,
            RECIPIENT,
            100,
            &BlockInfo::default(),
            &mut None,
            None,
            platform_version,
        )?);
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    #[test]
    fn should_create_the_distribution_storage_of_tokens_added_by_update_before_the_upgrade() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Two contracts releasing at the same time share that time's timed distribution tree.
        let token_ids = [1u8, 2].map(|contract_seed| {
            add_token_by_update_before_the_upgrade(
                &drive,
                contract_seed,
                token_with_both_distributions(445),
            )
        });
        for token_id in token_ids {
            record_both_claims(&drive, token_id)
                .expect_err("no claim can be recorded before the upgrade");
        }

        assert_eq!(run_backfill(&drive), 2);

        for token_id in token_ids {
            let distributions = drive
                .fetch_token_pre_programmed_distributions(
                    token_id,
                    None,
                    None,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the pre-programmed distributions");
            assert_eq!(
                distributions,
                BTreeMap::from([(100, BTreeMap::from([(Identifier::from(RECIPIENT), 445)]))])
            );

            record_both_claims(&drive, token_id)
                .expect("both claims should be recordable after the upgrade");
        }
    }

    #[test]
    fn should_leave_tokens_that_already_have_their_storage_unchanged() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Registered with its token: the contract insert created the storage.
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_id([3; 32].into());
        contract.set_tokens(BTreeMap::from([(0, token_with_both_distributions(445))]));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to insert the contract with its token");

        add_token_by_update_before_the_upgrade(&drive, 4, token_with_both_distributions(445));

        assert_eq!(run_backfill(&drive), 1, "only the token added by update");

        // A retried upgrade block finds nothing left to do.
        let root_hash_after_backfill = root_hash(&drive);
        assert_eq!(run_backfill(&drive), 0);
        assert_eq!(root_hash(&drive), root_hash_after_backfill);
    }

    /// No validation bounds a pre-programmed amount, and before protocol version 14 an update
    /// never wrote the release, so state can hold a release no sum tree can. The upgrade block
    /// must not fail over it.
    #[test]
    fn should_skip_a_pre_programmed_distribution_that_can_not_be_stored() {
        let drive = setup_drive_with_initial_state_structure(None);

        let token_id = add_token_by_update_before_the_upgrade(
            &drive,
            5,
            token_with_both_distributions(u64::MAX),
        );

        assert_eq!(run_backfill(&drive), 1, "the perpetual storage is created");

        let has_pre_programmed_storage = drive
            .grove_has_raw(
                (&token_root_pre_programmed_distributions_path()).into(),
                &token_id,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to look for the pre-programmed storage");
        assert!(
            !has_pre_programmed_storage,
            "the pre-programmed storage is not"
        );
    }
}
