use crate::platform::transition::address_inputs::nonce_inc;
use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::{Error, Sdk};

use super::address_inputs::fetch_inputs_with_nonce;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::{AddressNonce, AssetLockProof, Identity};
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransition;
use simple_signer::SimpleAddressSigner;
use std::collections::BTreeMap;

/// Funding sources supported when creating an identity.
pub enum IdentityFunding {
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    },
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    },
}

/// A trait for putting an identity to platform
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer<IdentityPublicKey>>: Waitable {
    /// Puts an identity on platform.
    ///
    /// TODO: Discuss if it should not actually consume self, since it is no longer valid (eg. identity id is changed)
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Sends the identity and waits for confirmation proof.
    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

    /// Deprecated alias for [`send_to_platform`].
    #[deprecated(note = "use send_to_platform instead")]
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        self.send_to_platform(
            sdk,
            IdentityFunding::AssetLock {
                asset_lock_proof,
                asset_lock_private_key: *asset_lock_proof_private_key,
            },
            signer,
            settings,
        )
        .await
    }

    /// Deprecated alias for [`send_to_platform_and_wait_for_response`].
    #[deprecated(note = "use send_to_platform_and_wait_for_response instead")]
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        self.send_to_platform_and_wait_for_response(
            sdk,
            IdentityFunding::AssetLock {
                asset_lock_proof,
                asset_lock_private_key: *asset_lock_proof_private_key,
            },
            signer,
            settings,
        )
        .await
    }
}
#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutIdentity<S> for Identity {
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        send_to_identity_with_source(self, sdk, funding, signer, settings).await
    }

    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition =
            send_to_identity_with_source(self, sdk, funding, signer, settings).await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

async fn send_to_identity_with_source<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    funding: IdentityFunding,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    match &funding {
        IdentityFunding::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        } => {
            let (state_transition, _) = identity.broadcast_request_for_new_identity(
                asset_lock_proof.to_owned(),
                asset_lock_private_key,
                signer,
                sdk.version(),
            )?;
            ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
            state_transition.broadcast(sdk, settings).await?;
            Ok(state_transition)
        }
        IdentityFunding::Addresses {
            inputs,
            input_private_keys,
        } => {
            let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
            send_identity_with_addresses(
                identity,
                sdk,
                inputs_with_nonce,
                input_private_keys,
                signer,
                settings,
            )
            .await
        }
        IdentityFunding::AddressesWithNonce {
            inputs,
            input_private_keys,
        } => {
            send_identity_with_addresses(
                identity,
                sdk,
                inputs.clone(),
                input_private_keys,
                signer,
                settings,
            )
            .await
        }
    }
}

/// A simple signer for platform addresses that maps addresses to their private keys
#[derive(Debug)]
struct SimpleAddressSigner {
    /// Maps address hash (20 bytes) to private key (32 bytes)
    keys: BTreeMap<[u8; 20], [u8; 32]>,
}

impl SimpleAddressSigner {
    /// Create a new address signer from addresses and their corresponding private keys
    fn new(addresses: &[PlatformAddress], private_keys: &[Vec<u8>]) -> Result<Self, ProtocolError> {
        if addresses.len() != private_keys.len() {
            return Err(ProtocolError::Generic(
                "Number of addresses must match number of private keys".to_string(),
            ));
        }

        let secp = Secp256k1::new();
        let mut keys = BTreeMap::new();

        for (address, private_key) in addresses.iter().zip(private_keys.iter()) {
            if private_key.len() != 32 {
                return Err(ProtocolError::Generic(
                    "Private key must be 32 bytes".to_string(),
                ));
            }

            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(private_key);

            // Verify the private key corresponds to this address
            let secret_key = SecretKey::from_byte_array(&key_bytes)
                .map_err(|e| ProtocolError::Generic(format!("Invalid private key: {}", e)))?;
            let public_key =
                dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
            let pubkey_hash: [u8; 20] =
                hash160::Hash::hash(&public_key.serialize()).to_byte_array();

            let address_hash = match address {
                PlatformAddress::P2pkh(hash) => *hash,
                PlatformAddress::P2sh(_) => {
                    return Err(ProtocolError::Generic(
                        "P2SH addresses not yet supported in SimpleAddressSigner".to_string(),
                    ));
                }
            };

            if pubkey_hash != address_hash {
                return Err(ProtocolError::Generic(
                    "Private key does not match address".to_string(),
                ));
            }

            keys.insert(address_hash, key_bytes);
        }

        Ok(Self { keys })
    }
}

impl Signer<PlatformAddress> for SimpleAddressSigner {
    fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let hash = match key {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(_) => {
                return Err(ProtocolError::Generic(
                    "P2SH addresses not supported".to_string(),
                ));
            }
        };

        let private_key = self.keys.get(hash).ok_or_else(|| {
            ProtocolError::Generic(format!("No private key found for address {:?}", key))
        })?;

        let signature = signer::sign(data, private_key)?;
        Ok(signature.to_vec().into())
    }

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data)?;
        match key {
            PlatformAddress::P2pkh(_) => Ok(AddressWitness::P2pkh { signature }),
            PlatformAddress::P2sh(_) => Err(ProtocolError::Generic(
                "P2SH addresses not supported".to_string(),
            )),
        }
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self.keys.contains_key(hash),
            PlatformAddress::P2sh(_) => false,
        }
    }
}

async fn send_identity_with_addresses<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    input_private_keys: &Vec<Vec<u8>>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    if input_private_keys.is_empty() {
        return Err(Error::Generic(
            "input_private_keys must contain at least one key".to_string(),
        ));
    }

    // Create address signer from inputs and private keys
    let addresses: Vec<PlatformAddress> = inputs.keys().cloned().collect();
    let address_signer =
        SimpleAddressSigner::from_addresses_and_keys(&addresses, input_private_keys)?;

    // Default fee strategy: deduct from first input
    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

    let user_fee_increase = settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
        .unwrap_or_default();

    let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
        identity,
        inputs,
        fee_strategy,
        signer,
        &address_signer,
        user_fee_increase,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    state_transition.broadcast(sdk, settings).await?;
    Ok(state_transition)
}
