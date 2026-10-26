use crate::drive::asset_lock_outpoint_info_query::asset_lock_outpoint_info_query;
use crate::drive::Drive;
use crate::error::Error;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_asset_lock_outpoint_info_v0(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.grove_get_proved_path_query(
            &asset_lock_outpoint_info_query(outpoint),
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
