use crate::drive::grove_operations::DirectQueryType;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;
use path::SubtreePath;

impl Drive {
    /// grove_get_direct_u64 is a helper function to get a
    pub(super) fn grove_get_raw_value_u64_from_encoded_var_vec_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<u64>, Error> {
        let element = self.grove_get_raw_optional(
            path,
            key,
            direct_query_type,
            transaction,
            drive_operations,
            drive_version,
        )?;
        element
            .map(|element| match element {
                Element::Item(value, ..) => u64::decode_var(value.as_slice())
                    .ok_or(Error::Drive(DriveError::CorruptedElementType(
                        "encoded value could not be decoded",
                    )))
                    .map(|(value, _)| value),
                Element::SumItem(value, ..) => Ok(value as u64),
                _ => Err(Error::Drive(DriveError::CorruptedQueryReturnedNonItem(
                    "expected an item",
                ))),
            })
            .transpose()
    }
}
