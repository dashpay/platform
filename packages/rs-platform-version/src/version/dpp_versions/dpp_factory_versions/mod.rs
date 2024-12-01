pub mod v1;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DPPFactoryVersions {
    pub data_contract_factory_structure_version: FeatureVersion,
    pub document_factory_structure_version: FeatureVersion,
}
