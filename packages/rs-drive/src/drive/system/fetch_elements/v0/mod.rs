use crate::drive::Drive;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// This function fetches elements at keys at a specific path
    ///
    /// # Parameters
    /// - `path`: The path at which we want to prove the elements
    /// - `keys`: The keys that we want to prove
    /// - `transaction`: An optional grovedb transaction
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    #[inline(always)]
    pub(super) fn fetch_elements_v0(
        &self,
        path: Vec<Vec<u8>>,
        keys: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Element>, Error> {
        let mut query = Query::default();
        query.insert_keys(keys);
        let path_query = PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };
        let (result, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryElementResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(result.to_elements())
    }
}
