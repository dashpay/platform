use crate::drive::votes::readiness::queries::readiness_fund_path_query;
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_readiness_fund_v0(
        &self,
        fund_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = readiness_fund_path_query(fund_id);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
