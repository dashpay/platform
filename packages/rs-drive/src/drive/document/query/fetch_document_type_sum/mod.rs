mod v0;

use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

impl Drive {
    /// Reads the total a document type keeps of its summed property over all its documents
    /// (`documentsSummable` or `documentsAverageable`), and returns the fee of the read.
    ///
    /// State validation reads it before a document create or replace: with values that are
    /// never negative every other sum of the type is a sum over some of the same documents,
    /// so a write that keeps this total in range keeps them all in range.
    ///
    /// # Parameters
    ///
    /// * `contract` - The data contract of the document type.
    /// * `document_type` - The document type, which must keep the total.
    /// * `epoch` - The epoch the read is priced at; `None` returns no fee.
    /// * `transaction` - The transaction to read in.
    /// * `platform_version` - The platform version selecting the method.
    ///
    /// # Returns
    ///
    /// The total and the fee of reading it. An error when the type keeps no total, or its
    /// primary tree is missing or is not a sum tree.
    pub fn fetch_document_type_sum(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(i128, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_type_sum
        {
            Some(0) => self.fetch_document_type_sum_v0(
                contract,
                document_type,
                epoch,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_type_sum".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_document_type_sum".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::paths::contract_document_type_path;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::tests::json_document::json_document_to_contract;
    use grovedb::operations::insert::InsertOptions;
    use grovedb::Element;

    fn setup_tip_jar() -> (Drive, DataContract) {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load the tip jar contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply the tip jar contract");
        (drive, contract)
    }

    #[test]
    fn should_read_the_total_of_a_type_with_no_documents_as_zero() {
        let (drive, contract) = setup_tip_jar();
        let document_type = contract
            .document_type_for_name("tip")
            .expect("expected the tip type");
        let (total, fee) = drive
            .fetch_document_type_sum(
                &contract,
                document_type,
                Some(&BlockInfo::default().epoch),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to read the total");
        assert_eq!(total, 0);
        assert!(fee.processing_fee > 0, "the read is priced");
    }

    /// An element that holds no total is not read as a total of 0, which would admit a
    /// write the type's sums may not hold.
    #[test]
    fn should_refuse_a_primary_tree_that_is_not_a_sum_tree() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup_tip_jar();
        let document_type = contract
            .document_type_for_name("tip")
            .expect("expected the tip type");
        let document_type_path =
            contract_document_type_path(contract.id_ref().as_bytes(), document_type.name());
        drive
            .grove
            .insert(
                &document_type_path,
                &[0],
                Element::empty_count_tree(),
                Some(InsertOptions {
                    validate_insertion_does_not_override: false,
                    validate_insertion_does_not_override_tree: false,
                    ..Default::default()
                }),
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to put a count tree in place of the primary tree");

        let error = drive
            .fetch_document_type_sum(&contract, document_type, None, None, platform_version)
            .expect_err("a count tree holds no total");
        assert!(
            matches!(error, Error::Drive(DriveError::CorruptedElementType(_))),
            "expected a corrupted element type, got {error:?}"
        );
    }
}
