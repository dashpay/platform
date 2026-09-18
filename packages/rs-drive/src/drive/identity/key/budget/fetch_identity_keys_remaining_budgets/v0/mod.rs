use crate::drive::Drive;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_identity_keys_remaining_budgets_v0(
        &self,
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, Option<Credits>>, Error> {
        key_ids
            .iter()
            .map(|key_id| {
                Ok((
                    *key_id,
                    self.fetch_identity_key_remaining_budget(
                        identity_id,
                        *key_id,
                        transaction,
                        platform_version,
                    )?,
                ))
            })
            .collect()
    }
}
