use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::identity::KeyOfTypeWithNonce;
use grovedb::element::SumValue;
use grovedb::Element;

impl Drive {
    /// Version 0 implementation of setting a balance for an address.
    /// This operation directly sets (or overwrites) the balance for a given address in the AddressBalances tree.
    ///
    /// # Parameters
    /// * `key_of_type_with_nonce`: The key (containing key type and key data) with its associated nonce
    /// * `balance`: The balance value to set
    /// * `drive_operations`: The list of drive operations to append to.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn set_balance_to_address_v0(
        &self,
        key_of_type_with_nonce: KeyOfTypeWithNonce,
        balance: Credits,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let KeyOfTypeWithNonce { key_of_type, nonce } = key_of_type_with_nonce;

        let key_of_type_bytes = key_of_type;
        let path = vec![vec![RootTree::AddressBalances as u8]];

        // Simply insert/overwrite the balance as an ItemWithSumItem element
        // The nonce is stored as big-endian bytes, and the balance is the sum value
        drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            path,
            key_of_type_bytes.to_bytes(),
            Element::new_item_with_sum_item_with_flags(
                nonce.to_be_bytes().to_vec(),
                balance as SumValue,
                None,
            ),
        ));

        Ok(())
    }
}
