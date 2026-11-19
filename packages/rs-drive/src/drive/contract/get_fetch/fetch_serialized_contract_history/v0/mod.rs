use crate::util::common::decode::decode_u64;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::data_contract::DataContract;

use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::{Element, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    /// [`Drive::fetch_contract_with_history`]'s revisions, each serialized as
    /// [`DataContract::carried_bytes_from_storage_bytes`] gives it, without parsing it.
    #[inline(always)]
    pub(super) fn fetch_serialized_contract_history_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        start_at_date: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u64, Vec<u8>>, Error> {
        let mut ops = Vec::new();

        let path_query =
            Self::fetch_contract_history_query(contract_id, start_at_date, limit, offset)?;

        let (results, _cost) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut ops,
            &platform_version.drive,
        )?;

        results
            .elements
            .iter()
            .map(|el| match el {
                QueryResultElement::KeyElementPairResultItem((key, Element::Item(item, _))) => {
                    let contract_time = decode_u64(key).map_err(|_| {
                        Error::Drive(DriveError::CorruptedContractPath(
                            "contract key is not a valid u64",
                        ))
                    })?;
                    let serialized =
                        DataContract::carried_bytes_from_storage_bytes(item, platform_version)?;
                    Ok((contract_time, serialized))
                }
                _ => Err(Error::Drive(DriveError::CorruptedContractPath(
                    "contract path did not refer to a contract element",
                ))),
            })
            .collect()
    }
}
