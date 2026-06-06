use crate::drive::document::paths::contract_documents_keeping_history_primary_key_path_for_document_id;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentPropertyType;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_document_history_query_v0(
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
    ) -> Result<PathQuery, Error> {
        let limit = Self::validate_document_history_limit(limit)?;
        let encoded_start = DocumentPropertyType::encode_date_timestamp(start_at_ms);
        let query = Query::new_single_query_item_with_direction(
            QueryItem::RangeAfter(std::ops::RangeFrom {
                start: encoded_start,
            }),
            true,
        );
        let path = contract_documents_keeping_history_primary_key_path_for_document_id(
            contract_id.as_slice(),
            document_type_name,
            document_id.as_slice(),
        )
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect();

        Ok(PathQuery::new(
            path,
            SizedQuery::new(query, Some(limit), offset),
        ))
    }
}
