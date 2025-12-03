use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::{Address, PrivateKey};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::BinaryData;
use dpp::prelude::{AddressNonce, AssetLockProof};
use dpp::ProtocolError;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;
use std::sync::Arc;
use zeroize::Zeroize;

/// Trait-object alias for identity signers.
pub type DynIdentitySigner = dyn Signer<IdentityPublicKey> + Send + Sync;

/// Generic wrapper around dynamic signers.
#[derive(Clone)]
pub struct TransferSigner<T> {
    inner: Arc<dyn Signer<T> + Send + Sync>,
}

impl<T> TransferSigner<T> {
    /// Create a wrapper from a dynamic signer.
    pub fn new(inner: Arc<dyn Signer<T> + Send + Sync>) -> Self {
        Self { inner }
    }

    /// Clone the inner signer.
    pub fn as_arc(&self) -> Arc<dyn Signer<T> + Send + Sync> {
        Arc::clone(&self.inner)
    }
}

impl<T> fmt::Debug for TransferSigner<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransferSigner").finish()
    }
}

impl<T> From<Arc<dyn Signer<T> + Send + Sync>> for TransferSigner<T> {
    fn from(inner: Arc<dyn Signer<T> + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl<T, S> From<Arc<S>> for TransferSigner<T>
where
    S: Signer<T> + Send + Sync + 'static,
{
    fn from(signer: Arc<S>) -> Self {
        let inner: Arc<dyn Signer<T> + Send + Sync> = signer;
        Self { inner }
    }
}

impl<T> From<TransferSigner<T>> for Arc<dyn Signer<T> + Send + Sync> {
    fn from(wrapper: TransferSigner<T>) -> Self {
        wrapper.inner
    }
}

impl<T> Signer<T> for TransferSigner<T> {
    fn sign(&self, key: &T, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.inner.sign(key, data)
    }

    fn sign_create_witness(&self, key: &T, data: &[u8]) -> Result<AddressWitness, ProtocolError> {
        self.inner.sign_create_witness(key, data)
    }

    fn can_sign_with(&self, key: &T) -> bool {
        self.inner.can_sign_with(key)
    }
}

/// Wrapper used for identity signers exposed via the builder API.
pub type IdentitySigner = TransferSigner<IdentityPublicKey>;
/// Wrapper used for Platform address signers exposed via the builder API.
pub type AddressSigner = TransferSigner<PlatformAddress>;

/// Configuration describing an identity funding source.
#[derive(Clone)]
pub struct IdentityTransferConfig {
    /// Identity funding the transfer.
    pub(crate) identity: Identity,
    /// Signer used for authorization.
    pub(crate) signer: IdentitySigner,
    /// Optional key override used for signing.
    pub(crate) signing_key: Option<IdentityPublicKey>,
}

impl IdentityTransferConfig {
    /// Create a new configuration for the provided identity and signer.
    pub fn new<S>(identity: Identity, signer: S, signing_key: Option<IdentityPublicKey>) -> Self
    where
        S: Into<IdentitySigner>,
    {
        Self {
            identity,
            signer: signer.into(),
            signing_key,
        }
    }

    /// Return the identity identifier.
    pub fn identity_id(&self) -> Identifier {
        self.identity.id()
    }

    /// Clone the signer.
    pub(crate) fn signer(&self) -> Arc<DynIdentitySigner> {
        self.signer.as_arc()
    }

    /// Borrow the preferred signing key if provided.
    pub(crate) fn signing_key(&self) -> Option<&IdentityPublicKey> {
        self.signing_key.as_ref()
    }

    /// Borrow the underlying identity.
    pub(crate) fn identity(&self) -> &Identity {
        &self.identity
    }
}

/// Generic funding sources for credit-backed transitions.
#[allow(private_interfaces)]
pub enum TransferInput {
    /// Asset-lock proof paired with its private key.
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    /// Platform address inputs without nonce information.
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    },
    /// Platform address inputs with nonce information.
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    },
    /// Identity source containing signer metadata.
    Identity(IdentityTransferConfig),
}

impl Zeroize for TransferInput {
    fn zeroize(&mut self) {
        match self {
            TransferInput::AssetLock {
                asset_lock_private_key,
                ..
            } => asset_lock_private_key.inner.non_secure_erase(),
            TransferInput::Addresses {
                input_private_keys, ..
            }
            | TransferInput::AddressesWithNonce {
                input_private_keys, ..
            } => input_private_keys.zeroize(),
            TransferInput::Identity(_) => {}
        }
    }
}

impl Drop for TransferInput {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl TransferInput {
    /// Build an asset-lock funding source.
    pub fn from_asset_lock(
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    ) -> Self {
        Self::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        }
    }

    /// Build a Platform address funding source without nonce.
    pub fn from_addresses(
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    ) -> Self {
        Self::Addresses {
            inputs,
            input_private_keys,
        }
    }

    /// Build a Platform address funding source that carries nonce information.
    pub fn from_addresses_with_nonce(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    ) -> Self {
        Self::AddressesWithNonce {
            inputs,
            input_private_keys,
        }
    }
}

impl From<IdentityTransferConfig> for TransferInput {
    fn from(value: IdentityTransferConfig) -> Self {
        TransferInput::Identity(value)
    }
}

impl From<(AssetLockProof, PrivateKey)> for TransferInput {
    fn from(value: (AssetLockProof, PrivateKey)) -> Self {
        Self::from_asset_lock(value.0, value.1)
    }
}

impl From<(BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)> for TransferInput {
    fn from(value: (BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)) -> Self {
        Self::from_addresses(value.0, value.1)
    }
}

impl
    From<(
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        Vec<Vec<u8>>,
    )> for TransferInput
{
    fn from(
        value: (
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
            Vec<Vec<u8>>,
        ),
    ) -> Self {
        Self::from_addresses_with_nonce(value.0, value.1)
    }
}

/// Destination variants for credit transfers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferOutput {
    /// Send credits to another identity.
    Identity(Identifier),
    /// Send credits to a Platform address.
    PlatformAddress(PlatformAddress),
    /// Send credits to a Core script.
    CoreScript(Vec<u8>),
    /// Send credits to the default withdrawal destination.
    DefaultWithdrawal,
}

impl TransferOutput {
    /// Helper constructing from raw script bytes.
    fn from_core_script_bytes(bytes: Vec<u8>) -> Self {
        TransferOutput::CoreScript(bytes)
    }
}

impl TryFrom<Identifier> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: Identifier) -> Result<Self, Self::Error> {
        Ok(TransferOutput::Identity(value))
    }
}

impl TryFrom<&Identifier> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: &Identifier) -> Result<Self, Self::Error> {
        Ok(TransferOutput::Identity(*value))
    }
}

impl TryFrom<&Identity> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: &Identity) -> Result<Self, Self::Error> {
        Ok(TransferOutput::Identity(value.id()))
    }
}

impl TryFrom<Identity> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: Identity) -> Result<Self, Self::Error> {
        Ok(TransferOutput::Identity(value.id()))
    }
}

impl TryFrom<PlatformAddress> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: PlatformAddress) -> Result<Self, Self::Error> {
        Ok(TransferOutput::PlatformAddress(value))
    }
}

impl TryFrom<Address> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        Ok(TransferOutput::from_core_script_bytes(
            value.script_pubkey().into_bytes(),
        ))
    }
}

impl TryFrom<dpp::identity::core_script::CoreScript> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: dpp::identity::core_script::CoreScript) -> Result<Self, Self::Error> {
        Ok(TransferOutput::from_core_script_bytes(
            value.as_bytes().to_vec(),
        ))
    }
}

impl TryFrom<&dpp::identity::core_script::CoreScript> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: &dpp::identity::core_script::CoreScript) -> Result<Self, Self::Error> {
        Ok(TransferOutput::from_core_script_bytes(
            value.as_bytes().to_vec(),
        ))
    }
}

impl TryFrom<Option<Address>> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: Option<Address>) -> Result<Self, Self::Error> {
        Ok(match value {
            Some(address) => {
                TransferOutput::from_core_script_bytes(address.script_pubkey().into_bytes())
            }
            None => TransferOutput::DefaultWithdrawal,
        })
    }
}
