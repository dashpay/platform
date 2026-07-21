mod document_create_transition;
mod document_delete_transition;
mod document_purchase_transition;
mod document_replace_transition;
mod document_transfer_transition;
mod document_transition;
mod document_update_price_transition;
mod documents_batch_transition;

use crate::error::Error;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::system_data_contracts::dpns_contract;
use dpp::system_data_contracts::dpns_contract::v1::document_types::domain;

/// Rewrites the `records.identity` name record of a DPNS `domain` document to
/// the document's new owner. Does nothing for any other document type.
///
/// A username resolves through `records.identity`, not through the document's
/// `$ownerId`, and domain documents are immutable (`Replace` is rejected by a
/// data trigger). Without this rewrite a transferred or purchased username
/// would keep resolving to the previous owner's identity with no way for the
/// new owner to repoint it.
fn rewrite_dpns_domain_identity_record_to_new_owner(
    document: &mut Document,
    data_contract_id: Identifier,
    document_type_name: &str,
    new_owner_id: Identifier,
) -> Result<(), Error> {
    if data_contract_id != dpns_contract::ID || document_type_name != domain::NAME {
        return Ok(());
    }

    document
        .properties_mut()
        .entry(domain::properties::RECORDS.to_string())
        .or_insert_with(|| Value::Map(Vec::new()))
        .set_value(
            domain::properties::IDENTITY,
            Value::Identifier(new_owner_id.into_buffer()),
        )
        .map_err(Error::Value)
}
