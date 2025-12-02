use crate::Error;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::Address;
use dpp::dashcore::PrivateKey;
use dpp::errors::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::core_script::CoreScript;
use dpp::identity::Identity;
use dpp::prelude::{AddressNonce, AssetLockProof};
use std::collections::BTreeMap;
use std::convert::Infallible;
use zeroize::Zeroize;

/// Generic funding sources for credit-backed transitions.
pub enum TransferInput {
    /// Use an asset lock proof/private key pair.
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    /// Use balances held on Platform addresses (nonces fetched automatically).
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
    },
    /// Use balances held on Platform addresses with explicitly provided nonces.
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
    },
}

impl Zeroize for TransferInput {
    fn zeroize(&mut self) {
        match self {
            TransferInput::AssetLock {
                asset_lock_private_key,
                ..
            } => {
                asset_lock_private_key.inner.non_secure_erase();
            }
            TransferInput::Addresses {
                input_private_keys, ..
            } => {
                input_private_keys.zeroize();
            }
            TransferInput::AddressesWithNonce {
                input_private_keys, ..
            } => {
                input_private_keys.zeroize();
            }
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
    /// Existing identity will receive credits.
    Identity(Identifier),
    /// Platform address will receive credits.
    PlatformAddress(PlatformAddress),
    /// Credits will be withdrawn to a core script (P2PKH/P2SH, etc.).
    CoreScript(Vec<u8>),
    /// Identity withdrawal without explicit destination (Drive will apply defaults).
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

impl TryFrom<CoreScript> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: CoreScript) -> Result<Self, Self::Error> {
        Ok(TransferOutput::from_core_script_bytes(
            value.as_bytes().to_vec(),
        ))
    }
}

impl TryFrom<&CoreScript> for TransferOutput {
    type Error = Infallible;

    fn try_from(value: &CoreScript) -> Result<Self, Self::Error> {
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

/// Aggregated credit transfer description created via [`CreditTransferBuilder`].
pub struct CreditTransfer {
    inputs: Vec<TransferInput>,
    outputs: BTreeMap<TransferOutput, Credits>,
}

impl CreditTransfer {
    /// Creates a new builder instance.
    pub fn builder() -> CreditTransferBuilder {
        CreditTransferBuilder::default()
    }

    /// Funding sources participating in this transfer.
    pub fn inputs(&self) -> &[TransferInput] {
        &self.inputs
    }

    /// Outputs and aggregated credit amounts.
    pub fn outputs(&self) -> &BTreeMap<TransferOutput, Credits> {
        &self.outputs
    }

    /// Decompose the transfer into owned inputs and outputs.
    pub fn into_parts(self) -> (Vec<TransferInput>, BTreeMap<TransferOutput, Credits>) {
        (self.inputs, self.outputs)
    }
}

/// Builder used to configure `CreditTransfer` inputs and outputs.
#[derive(Default)]
pub struct CreditTransferBuilder {
    inputs: Vec<TransferInput>,
    outputs: BTreeMap<TransferOutput, Credits>,
}

impl CreditTransferBuilder {
    /// Adds a funding source to the transfer.
    pub fn input<S>(&mut self, source: S) -> Result<&mut Self, Error>
    where
        S: TryInto<TransferInput> + Send,
        <S as TryInto<TransferInput>>::Error: ToString,
    {
        let funding = source.try_into().map_err(|err| {
            Error::InvalidCreditTransfer(format!("Invalid funding source: {}", err.to_string()))
        })?;
        self.inputs.push(funding);
        Ok(self)
    }

    /// Adds an output destination with the specified amount.
    pub fn output<D>(&mut self, destination: D, amount: Credits) -> Result<&mut Self, Error>
    where
        D: TryInto<TransferOutput>,
        <D as TryInto<TransferOutput>>::Error: ToString,
    {
        if amount == 0 {
            return Err(Error::from(OutputBelowMinimumError::new(amount, 1)));
        }

        let transfer_output = destination.try_into().map_err(|err| {
            Error::InvalidCreditTransfer(format!("Invalid transfer output: {}", err.to_string()))
        })?;

        let entry = self.outputs.entry(transfer_output).or_insert(0);
        *entry = entry.saturating_add(amount);

        Ok(self)
    }

    /// Finalizes the builder and returns an immutable `CreditTransfer`.
    pub fn build(self) -> Result<CreditTransfer, Error> {
        if self.inputs.is_empty() {
            return Err(Error::from(TransitionNoInputsError::new()));
        }

        if self.outputs.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        Ok(CreditTransfer {
            inputs: self.inputs,
            outputs: self.outputs,
        })
    }
}
