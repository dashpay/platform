use crate::drive::contract::moderation::types::{
    encode_document_removal, estimated_document_removal_value_size, ContractDocumentRecords,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// A record is the document owner's id, the moderator's id, the removal time, the hash of
    /// the document, its restoration if any, the fields its type keeps, and the reason, under
    /// the document's id, flagged with `moderator_id`: the moderator that writes it pays for it.
    /// Nothing ever deletes it. It is replaced in place, as a suspension is, when a restore
    /// marks it restored and when a restored document is deleted again (see
    /// `add_contract_document_record_operations_v0` for how its flags follow).
    ///
    /// An estimate prices a replacement as a fresh insert of what it adds to the record it
    /// replaces (`replaced_record_size`), so never less than the bytes it adds: GroveDB's
    /// average-case replace assumes an item keeps its size and would price no storage for the
    /// restoration a restore adds, which the moderator's balance is then not checked against,
    /// and pricing the whole record would charge the estimate for the reason and the kept
    /// fields the record already holds.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_document_removal_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        removal: &ContractDocumentRemoval,
        replaced_record_size: Option<u32>,
        estimated_kept_fields_size: u32,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // What the document's type keeps of it was bounded by the document's own size limits,
        // far inside the record's length prefixes, so an encoding refusal is a code path that
        // lost that guarantee, not a moderator's mistake.
        let mut value = encode_document_removal(removal).map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "the fields a removal record keeps exceed what a record can hold",
            ))
        })?;
        if let (true, Some(replaced_record_size)) = (
            estimated_costs_only_with_layer_info.is_some(),
            replaced_record_size,
        ) {
            let added = value
                .len()
                .saturating_sub(usize::try_from(replaced_record_size).unwrap_or(usize::MAX));
            value = vec![0; added];
        }
        self.add_contract_document_record_operations_v0(
            contract_id,
            ContractDocumentRecords::Removals,
            document_type_name,
            document_id,
            value,
            replaced_record_size.is_some(),
            estimated_document_removal_value_size(estimated_kept_fields_size),
            moderator_id,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )
    }
}
