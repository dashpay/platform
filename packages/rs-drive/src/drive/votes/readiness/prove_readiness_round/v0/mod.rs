use crate::drive::votes::readiness::queries::{
    readiness_report_path_query, readiness_round_path_query, readiness_round_pointer_path_query,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, TransactionArg};

impl Drive {
    pub(super) fn prove_readiness_round_v0(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let pointer_query = readiness_round_pointer_path_query(contract_id);
        let path_query = match self.fetch_readiness_current_round_id_operations(
            contract_id,
            transaction,
            &mut vec![],
            platform_version,
        )? {
            Some(round_id) => {
                let round_query = readiness_round_path_query(contract_id, round_id);
                PathQuery::merge(
                    vec![&pointer_query, &round_query],
                    &platform_version.drive.grove_version,
                )?
            }
            None => pointer_query,
        };
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }

    pub(super) fn prove_readiness_report_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = readiness_report_path_query(contract_id, round_id, pro_tx_hash);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
