use dpp::address_funds::PlatformAddress;
use dpp::dashcore::{Address, PrivateKey};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::prelude::{AddressNonce, AssetLockProof};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use zeroize::Zeroize;

pub type DynIdentitySigner = dyn Signer<IdentityPublicKey> + Send + Sync;

#[derive(Clone)]
pub struct IdentityTransferSigner {
    inner: Arc<DynIdentitySigner>,
}

impl IdentityTransferSigner {
    pub fn new(inner: Arc<DynIdentitySigner>) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> Arc<DynIdentitySigner> {
        Arc::clone(&self.inner)
    }
}

impl std::fmt::Debug for IdentityTransferSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityTransferSigner").finish()
    }
}

#[derive(Clone)]
pub struct IdentityTransferConfig {
    pub(crate) identity: Identity,
    pub(crate) signer: IdentityTransferSigner,
    pub(crate) signing_key: Option<IdentityPublicKey>,
}

impl IdentityTransferConfig {
    pub fn new(
        identity: Identity,
        signer: Arc<DynIdentitySigner>,
        signing_key: Option<IdentityPublicKey>,
    ) -> Self {
        Self {
            identity,
            signer: IdentityTransferSigner::new(signer),
            signing_key,
        }
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity.id()
    }

    pub(crate) fn signer(&self) -> IdentityTransferSigner {
        self.signer.clone()
    }

    pub(crate) fn signing_key(&self) -> Option<&IdentityPublicKey> {
        self.signing_key.as_ref()
    }

    pub(crate) fn identity(&self) -> &Identity {
        &self.identity
    }
}

/// Generic funding sources for credit-backed transitions.
#[allow(private_interfaces)]
pub enum TransferInput {
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
    pub fn from_asset_lock(
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    ) -> Self {
        Self::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        }
    }

    pub fn from_addresses(
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    ) -> Self {
        Self::Addresses {
            inputs,
            input_private_keys,
        }
    }

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
    Identity(Identifier),
    PlatformAddress(PlatformAddress),
    CoreScript(Vec<u8>),
    DefaultWithdrawal,
}

impl TransferOutput {
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
