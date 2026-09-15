use crate::drive::contract::paths::{contract_root_path, CONTRACT_VERSION_KEY};
use crate::drive::contract::version_item::encode_contract_version;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use grovedb::{Element, Transaction};

impl Drive {
    /// Writes the version item (`[64, id] / 2`, the version as four big-endian bytes) of
    /// every contract in state.
    ///
    /// Runs once, on the first block of protocol version 14: from that version the storage
    /// writer keeps the item current on every contract create and update, and
    /// `getDataContractsLatestVersions` reads and proves it instead of the contracts, so every
    /// contract stored before the version needs one. Each item carries its contract's element
    /// flags, so it is accounted to the contract's owner like the contract itself. Writing an
    /// item a contract already has is a no-op, so the walk is safe to repeat.
    pub fn add_version_items_to_all_contracts(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut start_at = None;
        let mut contract_count = 0usize;

        loop {
            let page =
                self.fetch_contract_ids(start_at, u16::MAX, Some(transaction), platform_version)?;

            for contract_id in &page {
                self.add_version_item_to_contract(*contract_id, transaction, platform_version)?;
            }
            contract_count += page.len();

            match page.last() {
                Some(last_id) if page.len() == u16::MAX as usize => {
                    start_at = Some((*last_id, false));
                }
                _ => break,
            }
        }

        tracing::info!(
            contract_count,
            "wrote the version item of every contract in state"
        );

        Ok(())
    }

    fn add_version_item_to_contract(
        &self,
        contract_id: [u8; 32],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
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

        let element_flags = fetch_info
            .storage_flags
            .as_ref()
            .map(StorageFlags::to_element_flags);
        let version_element = Element::Item(
            encode_contract_version(fetch_info.contract.version()),
            element_flags,
        );
        let contract_root_path = contract_root_path(&contract_id);

        self.grove_insert(
            (&contract_root_path).into(),
            &[CONTRACT_VERSION_KEY],
            version_element,
            Some(transaction),
            None,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb_epoch_based_storage_flags::StorageFlags;

    const PLAIN_CONTRACT: &str = "tests/supporting_files/contract/family/family-contract.json";
    const HISTORY_CONTRACT: &str =
        "tests/supporting_files/contract/references/references_with_contract_history.json";

    fn apply(drive: &Drive, contract: &DataContract, platform_version: &PlatformVersion) {
        drive
            .apply_contract(
                contract,
                BlockInfo {
                    time_ms: 1000,
                    height: 100,
                    core_height: 10,
                    epoch: Default::default(),
                },
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
    }

    /// Stores contracts of both layouts under protocol version 13, where no version item is
    /// written.
    fn setup_pre_upgrade_contracts(drive: &Drive) -> Vec<DataContract> {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        [(1u8, false), (2, true), (3, false)]
            .iter()
            .map(|(seed, keeps_history)| {
                let fixture = if *keeps_history {
                    HISTORY_CONTRACT
                } else {
                    PLAIN_CONTRACT
                };
                let mut contract = json_document_to_contract(fixture, false, platform_version)
                    .expect("expected to load the contract fixture");
                contract.set_id([*seed; 32].into());
                contract.set_version(*seed as u32 + 4);
                if *keeps_history {
                    contract.config_mut().set_keeps_history(true);
                    contract.config_mut().set_readonly(false);
                }
                apply(drive, &contract, platform_version);
                contract
            })
            .collect()
    }

    fn stored_version(drive: &Drive, contract: &DataContract) -> Option<u32> {
        drive
            .fetch_contract_version(contract.id().to_buffer(), None, PlatformVersion::latest())
            .expect("expected to read the version item")
    }

    #[test]
    fn should_write_the_version_item_of_every_contract_stored_before_the_upgrade() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contracts = setup_pre_upgrade_contracts(&drive);
        for contract in &contracts {
            assert_eq!(
                stored_version(&drive, contract),
                None,
                "no item before the upgrade"
            );
        }

        let transaction = drive.grove.start_transaction();
        drive
            .add_version_items_to_all_contracts(&transaction, platform_version)
            .expect("expected the backfill to succeed");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        for contract in &contracts {
            assert_eq!(
                stored_version(&drive, contract),
                Some(contract.version()),
                "the item holds the stored version"
            );
        }
    }

    #[test]
    fn should_leave_contracts_that_already_have_the_item_unchanged() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut contracts = setup_pre_upgrade_contracts(&drive);

        // A contract written under protocol version 14 already carries its item.
        let upgraded = &mut contracts[0];
        upgraded.increment_version();
        apply(&drive, upgraded, platform_version);
        assert_eq!(stored_version(&drive, upgraded), Some(upgraded.version()));

        let root_hash_before_backfill = |drive: &Drive| {
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash")
        };

        let transaction = drive.grove.start_transaction();
        drive
            .add_version_items_to_all_contracts(&transaction, platform_version)
            .expect("expected the backfill to succeed");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        let root_hash_after_first = root_hash_before_backfill(&drive);

        let transaction = drive.grove.start_transaction();
        drive
            .add_version_items_to_all_contracts(&transaction, platform_version)
            .expect("expected the repeated backfill to succeed");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        assert_eq!(
            root_hash_before_backfill(&drive),
            root_hash_after_first,
            "repeating the backfill changes nothing"
        );
        for contract in &contracts {
            assert_eq!(stored_version(&drive, contract), Some(contract.version()));
        }
    }
}
