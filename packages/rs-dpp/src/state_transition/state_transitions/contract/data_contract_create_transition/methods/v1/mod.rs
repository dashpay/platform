use crate::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};

use crate::prelude::IdentityNonce;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub trait DataContractCreateTransitionMethodsV1 {
    /// Creates and signs a data contract create transition that also registers a contract group
    /// and/or adds the created contract, one of its document types, or one of its tokens to
    /// contract groups.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The contract to create; its id and owner are set from the identity and
    ///   nonce.
    /// * `identity_nonce` - The identity nonce, also used to derive the registered group's id.
    /// * `contract_group` - The contract group to register, if any. Its id is derived from the
    ///   identity id and the nonce.
    /// * `contract_group_memberships` - The parts of the created contract that join groups.
    /// * `identity` - A reference to a `PartialIdentity` object.
    /// * `key_id` - The id of the identity public key used for signing.
    /// * `signer` - The signer.
    /// * `platform_version` - The platform version to build for.
    /// * `feature_version` - A transition version other than the platform version's default.
    #[allow(clippy::too_many_arguments)]
    async fn new_from_data_contract_with_contract_group<S: Signer<IdentityPublicKey>>(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        contract_group: Option<ContractGroupRegistration>,
        contract_group_memberships: Vec<ContractGroupMembership>,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        platform_version: &PlatformVersion,
        feature_version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;
}
