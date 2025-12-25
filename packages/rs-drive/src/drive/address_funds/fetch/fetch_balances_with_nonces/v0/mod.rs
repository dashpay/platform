use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    /// Version 0 implementation of fetching balances and nonces for multiple addresses.
    /// This operation retrieves the balance and nonce for multiple addresses from the AddressBalances tree.
    ///
    /// # Parameters
    /// * `addresses`: An iterator over platform addresses to look up
    /// * `transaction`: The transaction argument for the operation.
    /// * `platform_version`: The platform version for GroveDB compatibility
    ///
    /// # Returns
    /// * `Ok(BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>)` - A map from addresses to optional (nonce, balance) pairs.
    ///   All input addresses are included in the result. Addresses that exist have `Some((nonce, balance))`,
    ///   addresses that don't exist have `None`.
    /// * `Err(Error)` if the operation fails or if any element type is corrupted
    pub(in crate::drive::address_funds) fn fetch_balances_with_nonces_v0<'a, I>(
        &self,
        addresses: I,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>, Error>
    where
        I: IntoIterator<Item = &'a PlatformAddress>,
    {
        let path_query = Drive::balances_for_clear_addresses_query(addresses);

        // Execute the query
        let mut drive_operations = vec![];
        let results = self.grove_get_path_query_with_optional(
            &path_query,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Parse results and collect into map
        results
            .into_iter()
            .map(|(_path, key_bytes, element_opt)| {
                // Deserialize the key back to PlatformAddress
                let address = PlatformAddress::from_bytes(&key_bytes)?;

                let value = match element_opt {
                    Some(Element::ItemWithSumItem(nonce_bytes, balance, _)) => {
                        // Validate balance is non-negative
                        if balance < 0 {
                            return Err(Error::Drive(DriveError::CorruptedSerialization(format!(
                                "balance cannot be negative: {}",
                                balance
                            ))));
                        }

                        // Parse the nonce from big-endian bytes
                        let nonce_array: [u8; 4] =
                            nonce_bytes.as_slice().try_into().map_err(|_| {
                                Error::Drive(DriveError::CorruptedSerialization(
                                    "nonce must be 4 bytes for a u32".to_string(),
                                ))
                            })?;

                        let nonce = AddressNonce::from_be_bytes(nonce_array);

                        Some((nonce, balance as Credits))
                    }
                    Some(_) => {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "expected ItemWithSumItem element type for address balance",
                        )));
                    }
                    None => None,
                };

                Ok((address, value))
            })
            .collect()
    }
}
