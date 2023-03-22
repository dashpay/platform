use std::collections::BTreeMap;

use dpp::contracts::withdrawals_contract;
use dpp::dashcore::{BlockHeader, consensus};
use dpp::document::document_stub::DocumentStub;
use dpp::document::generate_document_id;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{IdentityCreditWithdrawalTransition, IdentityCreditWithdrawalTransitionAction, Pooling};
use dpp::util::entropy_generator::generate;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation, WithdrawalOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, DocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
    fn to_high_level_drive_operations(self) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreditWithdrawalTransitionAction {
            version, identity_id, amount, core_fee_per_byte, pooling, output_script, revision
        } = self;

        let data_contract_id = *withdrawals_contract::CONTRACT_ID;

        let data_contract = self.;

        let document_type = data_contract.document_type_for_name(document_type)?;

        let mut document_id;

        loop {
            let document_entropy = generate()?;

            document_id = generate_document_id::generate_document_id(
                data_contract_id,
                &self.identity_id,
                &document_type,
                &document_entropy,
            );

            // TODO: fetch documents
            let documents: Vec<DocumentStub> = vec![]; // self
                // .state_repository
                // .fetch_documents(
                //     withdrawals_contract::CONTRACT_ID.deref(),
                //     withdrawals_contract::document_types::WITHDRAWAL,
                //     json!({
                //         "where": [
                //             ["$id", "==", document_id],
                //         ],
                //     }),
                //     &state_transition.execution_context,
                // )
                // .await?;

            if documents.is_empty() {
                break;
            }
        }

        // TODO: fetch latest block header
        let latest_platform_block_header_bytes: Vec<u8> = vec![];

        let latest_platform_block_header: BlockHeader =
            consensus::deserialize(&latest_platform_block_header_bytes)?;

        let document_type = String::from(withdrawals_contract::document_types::WITHDRAWAL);
        let document_created_at_millis: i64 = latest_platform_block_header.time as i64 * 1000i64;

        let document_data = json!({
            withdrawals_contract::property_names::AMOUNT: state_transition.amount,
            withdrawals_contract::property_names::CORE_FEE_PER_BYTE: state_transition.core_fee_per_byte,
            withdrawals_contract::property_names::POOLING: Pooling::Never,
            withdrawals_contract::property_names::OUTPUT_SCRIPT: state_transition.output_script.as_bytes(),
            withdrawals_contract::property_names::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let mut document_properties = BTreeMap::new();

        // TODO: convert to values
        document_properties.insert(withdrawals_contract::property_names::AMOUNT, self.amount);
        document_properties.insert(withdrawals_contract::property_names::CORE_FEE_PER_BYTE, self.core_fee_per_byte);
        document_properties.insert(withdrawals_contract::property_names::POOLING, Pooling::Never);
        document_properties.insert(withdrawals_contract::property_names::OUTPUT_SCRIPT, self.output_script.as_bytes());
        document_properties.insert(withdrawals_contract::property_names::STATUS, withdrawals_contract::WithdrawalStatus::QUEUED);
        document_properties.insert(withdrawals_contract::property_names::CREATE_AT, document_created_at_millis);
        document_properties.insert(withdrawals_contract::property_names::UPDATED_AT, document_created_at_millis);

        let document = DocumentStub {
            id: document_id.to_buffer(),
            properties: document_properties,
            owner_id: self.identity_id.to_buffer(),
            revision: 0
        };

        let mut drive_operations = vec![];

        drive_operations.push(
            DocumentOperation(
                DocumentOperationType::AddDocumentForContract {
                    document_and_contract_info: DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentAndSerialization(
                                (document, document.to_cbor(), None)
                            ),
                            owner_id: None
                        },
                        contract: data_contract,
                        document_type: data_contract.document_type_for_name(document_type)?,
                    },
                    override_document: false,
                }
            )
        );

        drive_operations.push(IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance { identity_id: self.identity_id, balance_to_remove: self.amount }));
        drive_operations.push(DriveOperation::SystemOperation(crate::drive::batch::SystemOperationType::RemoveFromSystemCredits { amount: self.amount }));

        Ok(drive_operations)
    }
}
