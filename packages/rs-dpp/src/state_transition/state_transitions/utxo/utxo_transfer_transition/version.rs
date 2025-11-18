use crate::state_transition::utxo_transfer_transition::UTXOTransferTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for UTXOTransferTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            UTXOTransferTransition::V0(v0) => v0.feature_version(),
        }
    }
}
