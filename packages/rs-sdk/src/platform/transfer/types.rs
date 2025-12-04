use crate::Error;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::{Address, PrivateKey};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{core_script::CoreScript, Identity, IdentityPublicKey};
use dpp::prelude::{AddressNonce, AssetLockProof};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use zeroize::Zeroize;

/// Dynamic identity signer handle.
pub type IdentitySigner = Arc<dyn Signer<IdentityPublicKey> + Send + Sync>;
/// Dynamic Platform address signer handle.
pub type AddressSigner = Arc<dyn Signer<PlatformAddress> + Send + Sync>;

/// Builder-friendly signer configuration capable of holding either dynamic signers or raw keys.
#[derive(Clone, Debug)]
pub enum TransferSigner {
    /// Identity signer with optional preferred public key.
    Identity {
        signer: IdentitySigner,
        signing_key: Option<IdentityPublicKey>,
    },
    /// Platform address signer.
    Address(AddressSigner),
    /// Raw private keys (in order of the inputs supplied).
    PrivateKeys(Vec<Vec<u8>>),
}

impl TransferSigner {
    /// Backwards-compatible identity constructor.
    pub fn new(signer: IdentitySigner) -> Self {
        Self::new_identity_signer(signer)
    }

    /// Backwards-compatible identity constructor with explicit key.
    pub fn new_with_public_key(
        signer: IdentitySigner,
        signing_key: IdentityPublicKey,
    ) -> Self {
        Self::new_identity_signer_with_public_key(signer, signing_key)
    }

    /// Create an identity signer configuration.
    pub fn new_identity_signer(signer: IdentitySigner) -> Self {
        Self::Identity {
            signer,
            signing_key: None,
        }
    }

    /// Create an identity signer configuration with an explicit signing key.
    pub fn new_identity_signer_with_public_key(
        signer: IdentitySigner,
        signing_key: IdentityPublicKey,
    ) -> Self {
        Self::Identity {
            signer,
            signing_key: Some(signing_key),
        }
    }

    /// Wrap a Platform address signer handle.
    pub fn address_signer(signer: AddressSigner) -> Self {
        Self::Address(signer)
    }

    /// Store raw Platform address private keys for flows that require them.
    pub fn private_keys<I, K>(keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<Vec<u8>>,
    {
        Self::PrivateKeys(keys.into_iter().map(Into::into).collect())
    }
}

impl From<IdentitySigner> for TransferSigner {
    fn from(signer: IdentitySigner) -> Self {
        TransferSigner::Identity {
            signer,
            signing_key: None,
        }
    }
}

impl From<(IdentitySigner, IdentityPublicKey)> for TransferSigner {
    fn from((signer, signing_key): (IdentitySigner, IdentityPublicKey)) -> Self {
        TransferSigner::Identity {
            signer,
            signing_key: Some(signing_key),
        }
    }
}

impl From<AddressSigner> for TransferSigner {
    fn from(signer: AddressSigner) -> Self {
        TransferSigner::Address(signer)
    }
}

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

    /// Create a configuration from a [`TransferSigner`] helper.
    pub fn from_transfer_signer(identity: Identity, signer: TransferSigner) -> Result<Self, Error> {
        match signer {
            TransferSigner::Identity {
                signer,
                signing_key,
            } => Ok(Self {
                identity,
                signer,
                signing_key,
            }),
            TransferSigner::Address(_) | TransferSigner::PrivateKeys(_) => {
                Err(Error::InvalidCreditTransfer(
                    "identity transfer requires an identity signer configuration".to_string(),
                ))
            }
        }
    }

    /// Return the identity identifier.
    pub fn identity_id(&self) -> Identifier {
        self.identity.id()
    }

    /// Clone the signer.
    pub(crate) fn signer(&self) -> IdentitySigner {
        Arc::clone(&self.signer)
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
    /// Asset-lock proof.
    AssetLock {
        asset_lock_proof: AssetLockProof,
    },
    /// Platform address inputs without nonce information.
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
    },
    /// Platform address inputs with nonce information.
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    },
    /// Identity funding source.
    Identity(Identity),
}

impl Zeroize for TransferInput {
    fn zeroize(&mut self) {
        match self {
            TransferInput::AssetLock { .. }
            | TransferInput::Addresses { .. }
            | TransferInput::AddressesWithNonce { .. }
            | TransferInput::Identity(_) => {}
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
    ) -> Self {
        Self::AssetLock { asset_lock_proof }
    }

    /// Build a Platform address funding source without nonce.
    pub fn from_addresses(
        inputs: BTreeMap<PlatformAddress, Credits>,
    ) -> Self {
        Self::Addresses { inputs }
    }

    /// Build a Platform address funding source that carries nonce information.
    pub fn from_addresses_with_nonce(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Self {
        Self::AddressesWithNonce { inputs }
    }

    /// Build an identity funding source awaiting signer configuration.
    pub fn from_identity(identity: Identity) -> Self {
        Self::Identity(identity)
    }
}

impl From<(AssetLockProof, PrivateKey)> for TransferInput {
    fn from(value: (AssetLockProof, PrivateKey)) -> Self {
        Self::from_asset_lock(value.0)
    }
}

impl From<(BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)> for TransferInput {
    fn from(value: (BTreeMap<PlatformAddress, Credits>, Vec<Vec<u8>>)) -> Self {
        Self::from_addresses(value.0)
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
        Self::from_addresses_with_nonce(value.0)
    }
}

impl From<Identity> for TransferInput {
    fn from(identity: Identity) -> Self {
        TransferInput::from_identity(identity)
    }
}

impl From<&Identity> for TransferInput {
    fn from(identity: &Identity) -> Self {
        TransferInput::from_identity(identity.clone())
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

    /// Construct an output targeting a Core script.
    pub fn core_script(script: &CoreScript) -> Self {
        TransferOutput::from_core_script_bytes(script.as_bytes().to_vec())
    }

    /// Construct an output targeting a Core address.
    pub fn core_address(address: &Address) -> Self {
        TransferOutput::from_core_script_bytes(address.script_pubkey().into_bytes())
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
