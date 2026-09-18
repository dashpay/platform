use crate::drive::Drive;
use crate::error::Error;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_identity_keys_remaining_budgets_v0(
        &self,
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::identity_keys_remaining_budgets_query(identity_id, key_ids);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
