use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::Purpose;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves identities with all its information from an identity ids.
    pub(super) fn get_identities_contract_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let query = Self::identities_contract_keys_query(identity_ids, contract_id, document_type_name, purposes);
        let (keys, _) = self.grove_get_path_query_serialized_results(
            &query,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;

    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::GroveDb;
    use grovedb::QueryItem;
    use std::borrow::Borrow;
    use std::collections::BTreeMap;
    use std::ops::RangeFull;

    use crate::drive::Drive;

    use dpp::version::PlatformVersion;
}

