use crate::drive::credit_pools::paths::lifetime_storage_fee_pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::GroveError;
use dpp::balances::credits::Creditable;
use dpp::fee::fee_result::LifetimeStorageFees;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_lifetime_storage_fee_pools_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<LifetimeStorageFees, Error> {
        // One pool per number of epochs, at most one era of them.
        let path_query = PathQuery::new_unsized(
            lifetime_storage_fee_pools_vec_path(),
            Query::new_range_full(),
        );
        let pools = match self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok((pools, _)) => pools,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                return Ok(LifetimeStorageFees::new());
            }
            Err(e) => return Err(e),
        };
        pools
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let lifetime_epochs =
                    u16::from_be_bytes(key.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "a lifetime storage fee pool must be keyed by a u16".to_string(),
                        ))
                    })?);
                let Element::SumItem(credits, _) = element else {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "a lifetime storage fee pool must be a sum item",
                    )));
                };
                Ok((lifetime_epochs, credits.to_unsigned()))
            })
            .collect()
    }
}
