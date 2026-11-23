use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::schema::DefinitionsRoot;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::prelude::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Each document type of the contract, against the same type of `stored`, with the
    /// contract's `$defs` as it writes them, the first refusal reported on its type.
    #[inline(always)]
    pub(super) fn validate_gained_requirements_v0(
        &self,
        stored: Option<&DataContract>,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let definitions = DefinitionsRoot::new(self.schema_defs());
        for (name, document_type) in self.document_types() {
            let stored_type = stored
                .and_then(|stored| stored.document_types().get(name))
                .map(DocumentType::as_ref);
            if let Some(details) = document_type.as_ref().first_unmeetable_gained_requirement(
                stored_type,
                &definitions,
                self.version(),
                platform_version,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(name.clone(), details).into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }
}
