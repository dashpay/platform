use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::element::SumValue;
use grovedb::{Element, EstimatedLayerInformation};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Version 0 implementation of setting a balance for an address.
    /// This operation directly sets (or overwrites) the balance for a given address in the AddressBalances tree.
    ///
    /// # Parameters
    /// * `address`: The platform address
    /// * `nonce`: The nonce for the address
    /// * `balance`: The balance value to set
    /// * `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` - The operations to set balance.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn set_balance_to_address_operations_v0(
        &self,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let path = Self::clear_addresses_path();

        // Add estimation costs if we're in estimation mode
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_address_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        // Guard against u64 values that would overflow when cast to i64 (SumValue)
        if balance > i64::MAX as u64 {
            return Err(
                ProtocolError::Overflow("balance exceeds maximum for sum tree storage").into(),
            );
        }

        // Simply insert/overwrite the balance as an ItemWithSumItem element
        // The nonce is stored as big-endian bytes, and the balance is the sum value
        Ok(vec![
            LowLevelDriveOperation::insert_for_known_path_key_element(
                path,
                address.to_bytes(),
                Element::new_item_with_sum_item_with_flags(
                    nonce.to_be_bytes().to_vec(),
                    balance as SumValue,
                    None,
                ),
            ),
        ])
    }
}
