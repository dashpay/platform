use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use grovedb::element::SumValue;
use grovedb::Element;

impl Drive {
    /// Version 0 implementation of setting a balance for an address.
    /// This operation directly sets (or overwrites) the balance for a given address in the AddressBalances tree.
    ///
    /// # Parameters
    /// * `address`: The platform address
    /// * `nonce`: The nonce for the address
    /// * `balance`: The balance value to set
    /// * `drive_operations`: The list of drive operations to append to.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn set_balance_to_address_v0(
        &self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let path = Self::clear_addresses_path();

        // Simply insert/overwrite the balance as an ItemWithSumItem element
        // The nonce is stored as big-endian bytes, and the balance is the sum value
        drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            path,
            address.to_bytes(),
            Element::new_item_with_sum_item_with_flags(
                nonce.to_be_bytes().to_vec(),
                balance as SumValue,
                None,
            ),
        ));

        Ok(())
    }
}
