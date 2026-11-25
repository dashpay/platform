use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::is_transient;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_no_transient_summed_property_v0(&self) -> Result<(), ProtocolError> {
        let Some(property) = self
            .summed_properties()
            .into_iter()
            .find(|property| is_transient(*self, property))
        else {
            return Ok(());
        };
        Err(consensus_or_protocol_data_contract_error(
            DataContractError::InvalidContractStructure(format!(
                "document type \"{}\" sums \"{property}\", which it lists as transient: a \
                 transient value is never stored, and the type's sums read the stored value of \
                 each of its documents",
                self.name()
            )),
        ))
    }
}
