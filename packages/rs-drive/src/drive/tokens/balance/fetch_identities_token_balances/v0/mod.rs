use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_identities_token_balances_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        self.fetch_identities_token_balances_operations_v0(
            token_id,
            identity_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identities_token_balances_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        let path_query = Self::token_balances_for_identity_ids_query(token_id, identity_ids);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(_, key, element)| {
            let identity_id: Identifier = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "identity id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(SumItem(value, ..)) => Ok((identity_id, Some(value as TokenAmount))),
                None => Ok((identity_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for balances should contain only sum items".to_string(),
                ))),
            }
        })
        .collect()
    }
}
