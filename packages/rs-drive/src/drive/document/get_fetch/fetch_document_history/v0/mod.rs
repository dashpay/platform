use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::{QueryResultElement, QueryResultType};
use grovedb::{Element, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_document_history_v0(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u64, Document>, Error> {
        let mut ops = Vec::new();
        let path_query = Self::fetch_document_history_query(
            contract_id,
            document_type_name,
            document_id,
            start_at_ms,
            limit,
            offset,
            platform_version,
        )?;

        let (results, _cost) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut ops,
            &platform_version.drive,
        )?;

        results
            .elements
            .iter()
            .map(|el| match el {
                QueryResultElement::KeyElementPairResultItem((key, value)) => {
                    let document_time = DocumentPropertyType::decode_date_timestamp(key).ok_or(
                        Error::Drive(DriveError::CorruptedDocumentPath(
                            "document history key is not a valid timestamp",
                        )),
                    )?;
                    match value {
                        Element::Item(item, _flags) => {
                            let document =
                                Document::from_bytes(item, document_type, platform_version)
                                    .map_err(Error::from)?;
                            Ok((document_time, document))
                        }
                        _ => Err(Error::Drive(DriveError::CorruptedDocumentPath(
                            "document history path did not refer to a document item",
                        ))),
                    }
                }
                _ => Err(Error::Drive(DriveError::CorruptedDocumentPath(
                    "document history path did not refer to a key element pair",
                ))),
            })
            .collect()
    }
}
