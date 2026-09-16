mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::document::document_event::DocumentEvent;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    /// Adds a document event to the document history system contract, for
    /// document types that subscribed to history via the
    /// `keepsTransferHistory`, `keepsPurchaseHistory` and
    /// `keepsPricingHistory` configuration flags.
    #[allow(clippy::too_many_arguments)]
    pub fn add_document_history_operations(
        &self,
        source_data_contract_id: Identifier,
        source_document_type_name: &str,
        source_document_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: DocumentEvent,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_history_operations
        {
            0 => self.add_document_history_operations_v0(
                source_data_contract_id,
                source_document_type_name,
                source_document_id,
                owner_id,
                owner_nonce,
                event,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_history_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
