use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DPPCostsVersions {
    pub signature_verify: FeatureVersion,
}
