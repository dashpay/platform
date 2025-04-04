use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::token_event::TokenEvent;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds token transaction history
    #[allow(clippy::too_many_arguments)]
    pub fn add_token_transaction_history_operations(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: TokenEvent,
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
            .token
            .update
            .add_transaction_history_operations
        {
            0 => self.add_token_transaction_history_operations_v0(
                token_id,
                owner_id,
                owner_nonce,
                event,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_token_transaction_history_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
