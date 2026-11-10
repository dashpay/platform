use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use crate::serialization::Signable;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use crate::state_transition::StateTransition;
use crate::{NonConsensusError, ProtocolError};

impl DataContractUpdateTransitionV1 {
    /// Builds and signs the delta that turns `old_contract` into
    /// `new_contract`.
    ///
    /// # Arguments
    ///
    /// * `old_contract` - The contract as it is currently stored.
    /// * `new_contract` - The contract the update should produce.
    /// * `identity` - The `PartialIdentity` holding the signing key.
    /// * `key_id` - The key to sign with.
    /// * `identity_contract_nonce` - The next identity contract nonce.
    /// * `user_fee_increase` - The fee increase for priority processing.
    /// * `signer` - The `Signer` producing the signature.
    #[allow(clippy::too_many_arguments)]
    pub async fn new_from_contract_update<S: Signer<IdentityPublicKey>>(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
    ) -> Result<StateTransition, ProtocolError> {
        let mut transition = DataContractUpdateTransitionV1::from_contract_update(
            old_contract,
            new_contract,
            identity_contract_nonce,
        )?;
        transition.user_fee_increase = user_fee_increase;
        transition.signature_public_key_id = key_id;

        let mut state_transition: StateTransition = transition.into();
        let value = state_transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        state_transition.set_signature(signer.sign(public_key, &value).await?);
        Ok(state_transition)
    }
}
