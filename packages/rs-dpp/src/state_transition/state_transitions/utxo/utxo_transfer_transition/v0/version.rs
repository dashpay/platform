use crate::state_transition::utxo_transfer_transition::v0::UTXOTransferTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for UTXOTransferTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
