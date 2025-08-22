use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_tokens_direct_purchase_price_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::token_direct_purchase_prices_query(token_ids);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut Vec::new(),
            &platform_version.drive,
        )
    }
}
