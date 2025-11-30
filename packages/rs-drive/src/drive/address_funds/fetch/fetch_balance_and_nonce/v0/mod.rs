use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of fetching a balance and nonce for an address.
    /// This operation retrieves the balance and nonce for a given address from the AddressBalances tree.
    ///
    /// # Parameters
    /// * `address`: The platform address to look up
    /// * `transaction`: The transaction argument for the operation.
    ///
    /// # Returns
    /// * `Ok(Some((nonce, balance)))` if the address exists and has a balance
    /// * `Ok(None)` if the address does not exist
    /// * `Err(Error)` if the operation fails or if the element type is corrupted
    pub(in crate::drive::address_funds) fn fetch_balance_and_nonce_v0(
        &self,
        address: &PlatformAddress,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(AddressNonce, Credits)>, Error> {
        let path = vec![vec![RootTree::AddressBalances as u8]];
        let key_bytes = address.to_bytes();

        let mut drive_operations = vec![];

        // Get the element from the tree
        let element_opt = self.grove_get_raw_optional(
            path.as_slice().into(),
            &key_bytes,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        match element_opt {
            Some(Element::ItemWithSumItem(nonce_bytes, balance, _)) => {
                // Validate balance is non-negative
                if balance < 0 {
                    return Err(Error::Drive(DriveError::CorruptedSerialization(format!(
                        "balance cannot be negative: {}",
                        balance
                    ))));
                }

                // Parse the nonce from big-endian bytes
                let nonce_array: [u8; 4] = nonce_bytes.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "nonce must be 4 bytes for a u32".to_string(),
                    ))
                })?;

                let nonce = AddressNonce::from_be_bytes(nonce_array);

                Ok(Some((nonce, balance as Credits)))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "expected ItemWithSumItem element type for address balance",
            ))),
            None => Ok(None),
        }
    }
}
