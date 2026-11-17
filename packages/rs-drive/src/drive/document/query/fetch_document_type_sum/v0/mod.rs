use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

use crate::drive::document::paths::contract_document_type_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_document_type_sum_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(i64, FeeResult), Error> {
        if document_type.documents_summable().is_none() {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "only a document type that sums its documents keeps a total to fetch",
            )));
        }
        // The type's primary tree, key `0` of the type's tree: a sum tree whose element holds
        // the total of the summed property over the type's documents.
        let document_type_path =
            contract_document_type_path(contract.id_ref().as_bytes(), document_type.name());
        let mut operations: Vec<LowLevelDriveOperation> = vec![];
        let primary_tree = self
            .grove_get_raw_optional(
                (&document_type_path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                "a document type that sums its documents has no primary tree".to_string(),
            )))?;
        let mut element = &primary_tree;
        // A wrapper keeps the tree's total out of its parent's aggregates; the total is the
        // tree's own.
        while let Element::NonCounted(inner)
        | Element::NotSummed(inner)
        | Element::NotCountedOrSummed(inner) = element
        {
            element = inner;
        }
        // The tree types `primary_key_tree_type` gives a type that sums its documents. Any
        // other element holds no total, and reading it as 0 would admit a write the type's
        // sums may not hold.
        let total =
            match element {
                Element::SumTree(_, total, _)
                | Element::ProvableSumTree(_, total, _)
                | Element::CountSumTree(_, _, total, _)
                | Element::ProvableCountSumTree(_, _, total, _)
                | Element::ProvableCountProvableSumTree(_, _, total, _) => *total,
                _ => return Err(Error::Drive(DriveError::CorruptedElementType(
                    "the primary tree of a document type that sums its documents is not a sum tree",
                ))),
            };
        let fee = match epoch {
            Some(epoch) => self.calculate_fee(None, Some(operations), epoch, platform_version)?,
            None => FeeResult::default(),
        };
        Ok((total, fee))
    }
}
