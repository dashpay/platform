use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::document::lifecycle::fetch::DocumentLifecycleState;
use crate::drive::document::lifecycle::DocumentLifecycleRecord;
use crate::drive::document::paths::{
    contract_documents_primary_key_path, document_history_path, document_lifecycle_path,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{DirectQueryType, QueryType};

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_document_lifecycle_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_id: Identifier,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(DocumentLifecycleState, FeeResult), Error> {
        if !document_type.documents_keep_history() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "only a document type that keeps history has a lifecycle to fetch",
            )));
        }
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        let state = self.read_document_lifecycle(
            contract,
            document_type,
            document_id,
            &mut operations,
            transaction,
            platform_version,
        )?;
        let fee = match epoch {
            Some(epoch) => Drive::calculate_fee(
                None,
                Some(operations),
                epoch,
                self.config.epochs_per_era,
                platform_version,
                None,
            )?,
            None => FeeResult::default(),
        };
        Ok((state, fee))
    }

    fn read_document_lifecycle(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_id: Identifier,
        operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentLifecycleState, Error> {
        let primary_path = contract_documents_primary_key_path(
            contract.id_ref().as_bytes(),
            document_type.name().as_str(),
        );
        let pointer = self.grove_get_raw_optional(
            (&primary_path).into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            operations,
            &platform_version.drive,
        )?;
        if pointer.is_some() {
            let element = self
                .grove_get(
                    (&primary_path).into(),
                    document_id.as_slice(),
                    QueryType::StatefulQuery,
                    transaction,
                    operations,
                    &platform_version.drive,
                )?
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "a current pointer resolved to nothing".to_string(),
                )))?;
            let Element::Item(bytes, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "a current pointer did not resolve to a document",
                )));
            };
            return Ok(DocumentLifecycleState::Active(Box::new(
                Document::from_bytes(&bytes, document_type, platform_version)?,
            )));
        }

        let lifecycle_path =
            document_lifecycle_path(contract.id_ref().as_bytes(), document_type.name().as_str());
        // A document type whose documents have never been deleted has no
        // lifecycle tree at all, so a missing path means the same as a missing
        // key: the id is free.
        let record_bytes = self.grove_get_raw_optional_item(
            lifecycle_path.as_slice().into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            operations,
            &platform_version.drive,
        )?;
        let Some(record_bytes) = record_bytes else {
            return Ok(DocumentLifecycleState::Absent);
        };
        let record = DocumentLifecycleRecord::deserialize(&record_bytes)?;
        if record.is_erasing() {
            return Ok(DocumentLifecycleState::Erasing);
        }

        // A deleted document's owner is read from the newest revision it still
        // retains: a plain reverse range over the revision key space, immune to
        // the composite-key bounds a time selector has to get right.
        let mut query = Query::new_with_direction(false);
        query.insert_all();
        let path_query = PathQuery::new(
            document_history_path(
                contract.id_ref().as_bytes(),
                document_type.name().as_str(),
                document_id.as_slice(),
            ),
            SizedQuery::new(query, Some(1), None),
        );
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            operations,
            &platform_version.drive,
        )?;
        let Some((_, Element::Item(bytes, _))) = results.to_key_elements().into_iter().next()
        else {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "a deleted document retains no revision to read its owner from".to_string(),
            )));
        };
        Ok(DocumentLifecycleState::Deleted(Box::new(
            Document::from_bytes(&bytes, document_type, platform_version)?,
        )))
    }
}
