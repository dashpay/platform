use crate::drive::document::paths::contract_document_type_path;
use crate::drive::document::private_document_store::{
    private_document_entries_page_limit, private_document_of, PrivateDocumentEntries,
    PrivateDocumentEntry, PRIVATE_DOCUMENT_STORE_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_private_document_entries_v0(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        count: u32,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<PrivateDocumentEntries, Error> {
        let private_document = private_document_of(document_type)?;
        let type_path = contract_document_type_path(&contract_id, document_type.name());
        let chunk_power = self.committed_private_document_store_chunk_power(
            type_path,
            document_type.name(),
            transaction,
            platform_version,
        )?;
        let limit = private_document_entries_page_limit(chunk_power, start_position, count)?;
        let page = self
            .grove
            .private_document_store_get_range(
                type_path.as_slice(),
                &[PRIVATE_DOCUMENT_STORE_KEY],
                start_position,
                limit,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::from)?;
        let entries =
            page.entries
                .into_iter()
                .map(|(position, stored)| {
                    PrivateDocumentEntry::from_stored(position, &stored, private_document)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(format!(
                                "the entry at position {position} of the store of \"{}\" takes {} \
                             bytes, not its type's header and entry size",
                                document_type.name(),
                                stored.len()
                            )))
                        })
                })
                .collect::<Result<Vec<_>, Error>>()?;
        Ok(PrivateDocumentEntries {
            entries,
            total_count: page.total_count,
        })
    }
}
