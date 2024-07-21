use crate::drive::Drive;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// This function query requested identities, documents and contracts and provide cryptographic proofs
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
    pub(crate) fn prove_elements_v0(
        &self,
        path: Vec<Vec<u8>>,
        keys: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
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
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
