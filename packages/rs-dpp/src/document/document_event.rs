use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0};
use crate::fee::Credits;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::BTreeMap;

use crate::block::block_info::BlockInfo;

/// A document event that is recorded in the document history system contract
/// for document types that opted in via the `keepsTransferHistory`,
/// `keepsPurchaseHistory` and `keepsPricingHistory` configuration flags.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DocumentEvent {
    /// The document was transferred to another identity without a trade.
    /// The history document is owned by the sender.
    Transfer {
        /// The identity the document was transferred to
        to_identity_id: Identifier,
    },
    /// The document was bought at its asking price.
    /// The history document is owned by the buyer.
    Purchase {
        /// The identity that sold the document
        seller_id: Identifier,
        /// The price paid in credits
        price: Credits,
    },
    /// The document's asking price was updated by its owner.
    /// The history document is owned by the seller.
    PriceUpdate {
        /// The new asking price in credits
        price: Credits,
    },
}

impl DocumentEvent {
    /// The name of the document type in the document history contract that
    /// records this event.
    pub fn associated_document_type_name(&self) -> &'static str {
        match self {
            DocumentEvent::Transfer { .. } => "transfer",
            DocumentEvent::Purchase { .. } => "purchase",
            DocumentEvent::PriceUpdate { .. } => "priceUpdate",
        }
    }

    /// The document type in the document history contract that records this
    /// event.
    pub fn associated_document_type<'a>(
        &self,
        document_history_contract: &'a DataContract,
    ) -> Result<DocumentTypeRef<'a>, ProtocolError> {
        Ok(document_history_contract
            .document_type_for_name(self.associated_document_type_name())?)
    }

    /// Builds the history document recording this event.
    ///
    /// The id is deterministically derived from the source document, the
    /// acting identity and its contract nonce, so every validator produces
    /// the same history document for the same state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn build_historical_document_owned(
        self,
        source_data_contract_id: Identifier,
        source_document_type_name: &str,
        source_document_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        block_info: &BlockInfo,
    ) -> Document {
        let document_id = Document::generate_document_id_v0(
            &source_document_id,
            &owner_id,
            format!("history_{}", self.associated_document_type_name()).as_str(),
            owner_nonce.to_be_bytes().as_slice(),
        );

        let mut properties = BTreeMap::from([
            ("dataContractId".to_string(), source_data_contract_id.into()),
            (
                "documentTypeName".to_string(),
                source_document_type_name.into(),
            ),
            ("documentId".to_string(), source_document_id.into()),
        ]);

        match self {
            DocumentEvent::Transfer { to_identity_id } => {
                properties.insert("toIdentityId".to_string(), to_identity_id.into());
            }
            DocumentEvent::Purchase { seller_id, price } => {
                properties.insert("sellerId".to_string(), seller_id.into());
                properties.insert("price".to_string(), price.into());
            }
            DocumentEvent::PriceUpdate { price } => {
                properties.insert("price".to_string(), price.into());
            }
        }

        DocumentV0 {
            id: document_id,
            owner_id,
            properties,
            revision: None,
            created_at: Some(block_info.time_ms),
            updated_at: None,
            transferred_at: None,
            created_at_block_height: Some(block_info.height),
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into()
    }
}
