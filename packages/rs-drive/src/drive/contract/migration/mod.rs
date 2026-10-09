mod add_version_items_to_all_contracts;
mod rekey_unsigned_integer_index_values;
mod strip_unknown_document_schema_properties;

use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::Transaction;

impl Drive {
    /// Fetches every contract in state, in id order, and hands each to `each` with its id.
    /// Returns how many contracts there were. For the migrations of a protocol version's first
    /// block that walk every contract.
    fn for_each_contract_in_state(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        mut each: impl FnMut([u8; 32], &DataContractFetchInfo) -> Result<(), Error>,
    ) -> Result<usize, Error> {
        let mut start_at = None;
        let mut contract_count = 0usize;

        loop {
            let page =
                self.fetch_contract_ids(start_at, u16::MAX, Some(transaction), platform_version)?;

            for contract_id in &page {
                let fetch_info = self
                    .fetch_contract_and_add_operations(
                        *contract_id,
                        None,
                        Some(transaction),
                        &mut vec![],
                        platform_version,
                    )?
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "contract {} is listed under the contracts root but can not be \
                             fetched",
                            hex::encode(contract_id)
                        )))
                    })?;
                each(*contract_id, &fetch_info)?;
            }
            contract_count += page.len();

            match page.last() {
                Some(last_id) if page.len() == u16::MAX as usize => {
                    start_at = Some((*last_id, false));
                }
                _ => break,
            }
        }

        Ok(contract_count)
    }
}
