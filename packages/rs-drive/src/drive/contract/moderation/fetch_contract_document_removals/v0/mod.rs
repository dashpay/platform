use crate::drive::contract::moderation::types::{
    decode_document_removal, ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::contract::paths::{
    contract_document_removals_path, contract_document_type_removals_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_document_removals_v0(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractDocumentRemovalEntry>, Error> {
        Self::check_contract_document_removals_query(query, platform_version)?;

        // A path query over a missing tree is an error in GroveDB, so check first. The check
        // itself reads under the contract's removals tree, which a contract that declares no
        // moderation (or a contract id nobody has) does not have either: that reads as no
        // record, like the absence of the document type's own tree.
        let exists = match self.grove_has_raw(
            (&contract_document_removals_path(contract_id.as_slice())).into(),
            query.document_type_name.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(exists) => exists,
            Err(Error::GroveDB(error))
                if matches!(
                    *error,
                    grovedb::Error::PathParentLayerNotFound(_) | grovedb::Error::PathNotFound(_)
                ) =>
            {
                false
            }
            Err(error) => return Err(error),
        };
        if !exists {
            return Ok(vec![]);
        }

        let path_query = Self::contract_document_removals_query(contract_id.to_buffer(), query);
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                ContractDocumentRemovalEntry::from_key_element(&key, &element).map_err(
                    |description| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "contract {} {} document removal is malformed: {}",
                            contract_id, query.document_type_name, description
                        )))
                    },
                )
            })
            .collect()
    }

    /// One record, read the way the transform of a moderation reads state: the operations of
    /// the read are added to `drive_operations` for billing.
    #[inline(always)]
    pub(super) fn fetch_contract_document_removal_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractDocumentRemoval>, Error> {
        let path = contract_document_type_removals_path(contract_id.as_slice(), document_type_name);
        self.grove_get_raw_optional_item(
            (&path).into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .map(|value| {
            decode_document_removal(&value).map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} {} document {} removal is malformed: {}",
                    contract_id, document_type_name, document_id, description
                )))
            })
        })
        .transpose()
    }
}
