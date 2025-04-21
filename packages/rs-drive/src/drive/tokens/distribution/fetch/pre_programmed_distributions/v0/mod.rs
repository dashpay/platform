use crate::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches the pre‑programmed distributions for a token.
    ///
    /// This method queries the backing store for the pre‑programmed distributions tree at the path
    /// defined by `token_pre_programmed_distributions_path_vec(token_id)`. It then extracts a nested
    /// mapping where:
    ///
    /// - **Outer keys:** Are timestamps (`TimestampMillis`) representing each distribution time,
    ///   extracted from the 5th path component (index 4). The time is expected to be stored as 4 bytes in big‑endian.
    /// - **Inner keys:** Are recipient identifiers (`Identifier`) derived from the query key.
    /// - **Values:** Are token amounts (`TokenAmount`), extracted from elements that are sum items.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to use.
    ///
    /// # Returns
    ///
    /// A `Result` containing a nested `BTreeMap` on success or an `Error` on failure.
    pub(super) fn fetch_token_pre_programmed_distributions_operations_v0(
        &self,
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>, Error> {
        let path_query = Drive::pre_programmed_distributions_query(token_id, start_at, limit);

        let results = self
            .grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                drive_operations,
                &platform_version.drive,
            )?
            .0;

        let mut map: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>> = BTreeMap::new();

        for result_item in results.elements.into_iter() {
            if let QueryResultElement::PathKeyElementTrioResultItem((mut path, key, element)) =
                result_item
            {
                if let Some(last) = path.pop() {
                    if last.len() != 8 {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            format!("time key in pre-programmed distributions is not 8 bytes, got {} bytes instead", last.len()),
                        )));
                    }
                    let mut time_bytes = [0u8; 8];
                    time_bytes.copy_from_slice(last.as_slice());
                    let time = TimestampMillis::from_be_bytes(time_bytes);
                    let recipient = Identifier::from_bytes(key.as_slice())?;
                    let sum_item = element.as_sum_item_value()?;
                    if sum_item < 0 {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            "negative token amount in pre-programmed distribution".to_string(),
                        )));
                    }
                    let token_amount: TokenAmount = sum_item as TokenAmount;
                    map.entry(time).or_default().insert(recipient, token_amount);
                }
            }
        }

        Ok(map)
    }
}
