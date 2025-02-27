use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_token_statuses_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        self.fetch_token_statuses_operations_v0(
            token_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_token_statuses_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        let path_query = Drive::token_statuses_query(token_ids);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(_, key, element)| {
            let token_id: [u8; 32] = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "token id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(Item(value, ..)) => {
                    Ok((token_id, Some(TokenStatus::deserialize_from_bytes(&value)?)))
                }
                None => Ok((token_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for statuses should contain only items".to_string(),
                ))),
            }
        })
        .collect()
    }
}
