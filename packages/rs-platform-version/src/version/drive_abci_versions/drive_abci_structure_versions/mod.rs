pub mod v1;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStructureVersions {
    pub platform_state_structure: FeatureVersion,
    pub platform_state_for_saving_structure_default: FeatureVersion,
    pub reduced_platform_state_for_saving_structure_default: FeatureVersion,
    pub state_transition_execution_context: FeatureVersion,
    pub commit: FeatureVersion,
    pub masternode: FeatureVersion,
    pub signature_verification_quorum_set: FeatureVersion,
}
