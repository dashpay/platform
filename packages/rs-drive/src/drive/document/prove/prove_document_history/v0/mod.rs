use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prove_document_history_v0(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let history_query = Self::fetch_document_history_query(
            contract_id,
            document_type_name,
            document_id,
            start_at_ms,
            limit,
            offset,
            platform_version,
        )?;

        self.grove_get_proved_path_query(
            &history_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
