use crate::drive::tokens::{tokens_root_path_vec, TOKEN_IDENTITY_INFO_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_identity_token_infos_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        self.fetch_identity_token_infos_operations_v0(
            token_ids,
            identity_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_infos_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error> {
        let tokens_root = tokens_root_path_vec();

        let mut query = Query::new();

        for token_id in token_ids {
            query.insert_key(token_id.to_vec());
        }

        query.set_subquery_path(vec![vec![TOKEN_IDENTITY_INFO_KEY], identity_id.to_vec()]);

        let path_query = PathQuery::new(
            tokens_root,
            SizedQuery::new(query, Some(token_ids.len() as u16), None),
        );

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(path, _, element)| {
            let token_id: [u8; 32] = path
                .get(1)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "returned path item should always have a second part at index 1".to_string(),
                )))?
                .clone()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token id not 32 bytes".to_string(),
                    ))
                })?;
            match element {
                Some(Item(value, ..)) => Ok((
                    identity_id,
                    Some(IdentityTokenInfo::deserialize_from_bytes(&value)?),
                )),
                None => Ok((token_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for infos should contain only items".to_string(),
                ))),
            }
        })
        .collect()
    }
}
