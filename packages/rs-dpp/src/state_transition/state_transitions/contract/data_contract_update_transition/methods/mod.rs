mod registration_cost;
mod update_contract_cost;
mod v0;

pub use v0::*;

use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransition {
    /// Creates an update transition from a single data contract.
    ///
    /// Note: This method always creates a V0 transition (embedding the full contract)
    /// because V1 transitions require both old and new contracts to compute deltas.
    /// For V1 delta-based transitions, use `from_contract_update` instead.
    fn new_from_data_contract<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        // Always use V0 (embed full contract) since we only have a single contract.
        // V1 delta-based transitions require both old and new contracts.
        DataContractUpdateTransitionV0::new_from_data_contract(
            data_contract,
            identity,
            key_id,
            identity_contract_nonce,
            user_fee_increase,
            signer,
            platform_version,
            feature_version,
        )
    }
}
