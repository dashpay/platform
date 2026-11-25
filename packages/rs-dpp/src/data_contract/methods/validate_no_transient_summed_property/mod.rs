mod v0;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Refuses a contract one of whose document types sums a property it lists as transient
    /// (see [`DocumentTypeRef::validate_no_transient_summed_property`]).
    ///
    /// Contract registration and update run it on the contract they would store, once it is
    /// parsed, and refuse it with a paid consensus error. Parsing a contract does not run it,
    /// so a contract stored before the rule still loads, decodes with a document and parses
    /// under full validation, whatever its types sum.
    pub fn validate_no_transient_summed_property(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.document_types()
            .values()
            .try_for_each(|document_type| {
                document_type
                    .as_ref()
                    .validate_no_transient_summed_property(platform_version)
            })
    }
}

impl DocumentTypeRef<'_> {
    /// Refuses a document type that sums a property it lists as transient: Drive writes a
    /// document's value of the property into the type's sums when it stores the document, from
    /// the stored document's own properties, and a transient value is never stored. The
    /// refusal is an `InvalidContractStructure` consensus error.
    ///
    /// Registration and update refuse such a type through
    /// [`DataContract::validate_no_transient_summed_property`], and a document create of one a
    /// contract registered earlier holds is refused with the same error. A protocol version
    /// that predates the rule refuses nothing.
    pub fn validate_no_transient_summed_property(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_no_transient_summed_property
        {
            None => Ok(()),
            Some(0) => self.validate_no_transient_summed_property_v0(),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentTypeRef::validate_no_transient_summed_property".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
