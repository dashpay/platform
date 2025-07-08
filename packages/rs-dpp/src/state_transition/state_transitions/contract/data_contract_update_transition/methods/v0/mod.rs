use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub trait DataContractUpdateTransitionMethodsV0 {
    /// Creates a new instance of `DataContractUpdateTransition` from the given `data_contract`.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The `DataContract` to be used in the transition.
    /// * `identity` - A reference to the `PartialIdentity` containing the public keys.
    /// * `key_id` - The `KeyID` (public key identifier) to be used for signing the transition.
    /// * `signer` - A reference to the `Signer` object that will sign the transition.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - If successful, returns an instance of `DataContractUpdateTransition`.
    ///   In case of any error, a relevant `ProtocolError` is returned.
    #[allow(clippy::too_many_arguments)]
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
}
