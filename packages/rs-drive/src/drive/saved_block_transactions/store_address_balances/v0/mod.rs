use crate::drive::Drive;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::Element;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Version 0 implementation of storing address balance changes for a block.
    ///
    /// Serializes the address balance map using bincode and stores it in the
    /// SavedBlockTransactions tree keyed by block height.
    pub(super) fn store_address_balances_for_block_v0(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_height: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Early return if there are no balance changes to store
        if address_balances.is_empty() {
            return Ok(());
        }

        // Serialize the address balances map using bincode
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let serialized = bincode::encode_to_vec(address_balances, config).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode address balances: {}",
                e
            ))))
        })?;

        // Store in the SavedBlockTransactions/AddressBalances subtree with block height as key
        let path: [&[u8]; 2] = Drive::saved_block_transactions_address_balances_path();

        // Use block height as the key (big-endian for proper ordering)
        let key = block_height.to_be_bytes();

        // Insert the serialized data as an Item element
        let mut drive_operations = vec![];
        self.grove_insert(
            path.as_ref().into(),
            &key,
            Element::new_item(serialized),
            transaction,
            None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Apply any operations that were generated
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}
