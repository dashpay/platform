mod v0;

use crate::prelude::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Refuses a contract that gains an object with a `required` list no document could meet,
    /// or a document type whose parser reads a member as required without a list naming it
    /// (`DataContractInvalidRequiredFieldsUpdateError`). What a contract gains is every object
    /// a document type of a contract being registered reaches, at any depth and through `$ref`,
    /// and, in an update, every object of a type the update adds and every object it adds to a
    /// type the contract has (see
    /// [`DocumentTypeRef::first_unmeetable_gained_requirement`](crate::data_contract::document_type::DocumentTypeRef)).
    /// A `$defs` definition no type reaches is judged when a type gains a `$ref` to it.
    ///
    /// An object the contract has keeps its lists as they were (document type
    /// `validate_update` 2), so a contract stored before the rule stays updatable whatever its
    /// lists hold. Contract registration and update run it on the contract they would store,
    /// once it is parsed, and refuse it with a paid consensus error. A protocol version that
    /// predates the rule refuses nothing.
    ///
    /// # Parameters
    ///
    /// - `stored`: the contract as it is stored before an update, or `None` for a contract
    ///   being registered.
    /// - `platform_version`: selects the generation; `None` there refuses nothing.
    ///
    /// # Returns
    ///
    /// The first refusal, on the document type it is found in, or a valid result; an error only
    /// for a generation this build does not know.
    pub fn validate_gained_requirements(
        &self,
        stored: Option<&DataContract>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_gained_requirements
        {
            None => Ok(SimpleConsensusValidationResult::new()),
            Some(0) => Ok(self.validate_gained_requirements_v0(stored, platform_version)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_gained_requirements".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
