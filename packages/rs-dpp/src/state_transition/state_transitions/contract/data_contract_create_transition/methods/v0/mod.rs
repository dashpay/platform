use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};

use crate::prelude::IdentityNonce;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub trait DataContractCreateTransitionMethodsV0 {
    /// Creates a new instance of the DataContractCreateTransition from the provided data contract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A mutable `DataContract` instance, to be used in the transition.
    /// * `identity_nonce` - A `IdentityNonce` value providing additional randomness.
    /// * `identity` - A reference to a `PartialIdentity` object.
    /// * `key_id` - A `KeyID` identifier for the public key used for signing the transition.
    /// * `signer` - A reference to an object implementing the `Signer` trait.
    /// * `platform_version` - The current platform version that should be used.
    /// * `feature_version` - You can set the feature version to a different version than the default for the current
    ///   protocol version.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Result<Self, ProtocolError>` containing a `StateTransition`
    /// object. Otherwise, returns `ProtocolError`.
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
}
