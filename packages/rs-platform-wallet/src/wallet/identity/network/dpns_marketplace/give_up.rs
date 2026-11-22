//! Giving up a DPNS name a wallet identity holds, from protocol version 15: a delete, which
//! frees the name for anybody to register again, or a tombstone, which removes it and keeps
//! anybody from registering it again.
//!
//! Which of the two the network accepts for a name is the protocol's rule (a contested name
//! can only be tombstoned) and is read from the SDK, as is whether the network serves either
//! yet ([`dash_sdk::Sdk::dpns_name_give_up_options`]). A host offers what
//! [`IdentityWallet::dpns_name_give_up_choices`] reports and decides nothing of its own.
//!
//! The wallet forgets a name only once the network confirmed it gone: the identity's label
//! and the name's marketplace row are removed after the delete or the tombstone is proved,
//! never before. A give-up whose result was not confirmed leaves both, and the next
//! marketplace sync pass removes them when it finds the name gone.

use dash_sdk::platform::documents::transitions::DocumentDeleteResult;
use dpp::document::DocumentV0Getters;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::error::{promote_identity_insufficient_balance_or, PlatformWalletError};

use super::{dpns_contract_id, DpnsDomainState, IdentityWallet, DPNS_DOCUMENT_TYPE};

/// DPNS document type of a name given up for good, from DPNS v3.
const DPNS_TOMBSTONE_DOCUMENT_TYPE: &str = "tombstone";

/// The way the identity holding a DPNS name gives it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpnsNameGiveUpMethod {
    /// The name's `domain` document is deleted: the name is free again, and anybody may
    /// register it.
    Delete,
    /// A `tombstone` document takes the name's place: the name is gone and nobody can
    /// register it again. Nobody can undo it, the identity that created it included.
    Tombstone,
}

/// Which ways the network accepts for giving up one name a wallet identity holds
/// ([`IdentityWallet::dpns_name_give_up_choices`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameGiveUpChoices {
    /// The name's `domain` document id.
    pub document_id: Identifier,
    /// Display label, as the `domain` document carries it (e.g. "Alice").
    pub label: String,
    /// Whether [`DpnsNameGiveUpMethod::Delete`] is accepted. `false` for a contested name.
    pub can_delete: bool,
    /// Whether [`DpnsNameGiveUpMethod::Tombstone`] is accepted.
    pub can_tombstone: bool,
}

impl DpnsNameGiveUpChoices {
    /// Whether the network accepts `method` for this name.
    pub fn allows(&self, method: DpnsNameGiveUpMethod) -> bool {
        match method {
            DpnsNameGiveUpMethod::Delete => self.can_delete,
            DpnsNameGiveUpMethod::Tombstone => self.can_tombstone,
        }
    }
}

/// A name given up, as the network confirmed it ([`IdentityWallet::give_up_dpns_name`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameGivenUp {
    /// The id of the `domain` document that no longer exists.
    pub document_id: Identifier,
    /// Display label of the name given up.
    pub label: String,
    /// How it was given up.
    pub method: DpnsNameGiveUpMethod,
    /// The id of the tombstone the network stored, for [`DpnsNameGiveUpMethod::Tombstone`].
    pub tombstone_id: Option<Identifier>,
}

/// Map a give-up's SDK error. A protocol version refusal stays the SDK's own error, so a
/// host is told the network does not serve the feature yet rather than that the operation
/// failed; a balance refusal, an unavailable signing key and an unconfirmed result stay
/// typed too. Anything else is reported as the failed give-up of `name`.
fn give_up_error(error: dash_sdk::Error, name: &str) -> PlatformWalletError {
    if matches!(error, dash_sdk::Error::ProtocolVersionNotActive { .. }) {
        return PlatformWalletError::Sdk(error);
    }
    promote_identity_insufficient_balance_or(error, |error| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to give up DPNS name {name:?}: {error}"
        ))
    })
}

impl IdentityWallet {
    /// Which ways the network accepts for `owner_identity_id` to give up `name` ("alice" or
    /// "alice.dash"): a delete, a tombstone, or either. Nothing is signed or broadcast.
    ///
    /// # Errors
    ///
    /// - [`PlatformWalletError::Sdk`] holding the SDK's protocol version refusal when a
    ///   verified answer shows the network below protocol version 15, where no name can be
    ///   given up, or when the network's version could not be learned.
    /// - [`PlatformWalletError::DpnsNameNotFound`] when the name is not registered.
    /// - [`PlatformWalletError::InvalidParameter`] when another identity holds the name.
    pub async fn dpns_name_give_up_choices(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
    ) -> Result<DpnsNameGiveUpChoices, PlatformWalletError> {
        let options = self
            .sdk
            .dpns_name_give_up_options(name)
            .await
            .map_err(|error| give_up_error(error, name))?
            .ok_or_else(|| PlatformWalletError::DpnsNameNotFound {
                name: name.to_string(),
            })?;
        let owner_id = options.domain.owner_id();
        if owner_id != *owner_identity_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} is owned by {owner_id}, not by {owner_identity_id}"
            )));
        }
        let state = DpnsDomainState::from_document(&options.domain)?;
        Ok(DpnsNameGiveUpChoices {
            document_id: state.document_id,
            label: state.label,
            can_delete: options.can_delete,
            can_tombstone: options.can_tombstone,
        })
    }

    /// Give up `name` ("alice" or "alice.dash"), held by the wallet identity
    /// `owner_identity_id`, by `method`.
    ///
    /// Before anything is signed the name is read again: the network must serve the feature
    /// (protocol version 15), the name must be registered and held by `owner_identity_id`,
    /// and the network must accept `method` for it (a contested name can only be
    /// tombstoned). The signing key is selected on the owner, as for the other DPNS writes.
    ///
    /// The wallet's stored state changes only once the network confirmed the name gone: the
    /// label leaves the identity's name list, the name's marketplace row is removed, both
    /// through the persister, and the owner's balance is refreshed (a give-up refunds what
    /// is left of the name's storage fee). On any error nothing stored changes.
    pub async fn give_up_dpns_name<S>(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
        method: DpnsNameGiveUpMethod,
        signer: &S,
    ) -> Result<DpnsNameGivenUp, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let _operation = self.dpns_operation_gate.lock().await;
        let choices = self
            .dpns_name_give_up_choices(owner_identity_id, name)
            .await?;
        if !choices.allows(method) {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "the network does not accept {method:?} for DPNS name {name:?}: a contested \
                 name can only be tombstoned"
            )));
        }
        let tombstone_id = match method {
            DpnsNameGiveUpMethod::Delete => {
                let signing_key = self
                    .select_dpns_signing_key_for(owner_identity_id, DPNS_DOCUMENT_TYPE, signer)
                    .await?;
                let DocumentDeleteResult::Deleted(_) = self
                    .sdk
                    .delete_dpns_name(name, &signing_key, signer)
                    .await
                    .map_err(|error| give_up_error(error, name))?;
                None
            }
            DpnsNameGiveUpMethod::Tombstone => {
                let signing_key = self
                    .select_dpns_signing_key_for(
                        owner_identity_id,
                        DPNS_TOMBSTONE_DOCUMENT_TYPE,
                        signer,
                    )
                    .await?;
                let tombstone = self
                    .sdk
                    .tombstone_dpns_name(name, *owner_identity_id, signing_key, signer)
                    .await
                    .map_err(|error| give_up_error(error, name))?;
                Some(tombstone.id())
            }
        };
        self.forget_given_up_dpns_name(owner_identity_id, &choices)
            .await;
        self.refresh_identity_after_trade(owner_identity_id, "name give-up")
            .await;
        Ok(DpnsNameGivenUp {
            document_id: choices.document_id,
            label: choices.label,
            method,
            tombstone_id,
        })
    }

    /// Drop a name the network confirmed gone from the wallet's stored state: its
    /// marketplace row, then its label on `owner_identity_id`. The row goes first so that a
    /// host mirror which keeps one record per name sees the label leave a record that no
    /// longer carries marketplace history, and can drop the record whole.
    pub(super) async fn forget_given_up_dpns_name(
        &self,
        owner_identity_id: &Identifier,
        given_up: &DpnsNameGiveUpChoices,
    ) {
        self.record_dpns_name_states(vec![], vec![given_up.document_id])
            .await;
        self.remove_dpns_label(owner_identity_id, &given_up.label)
            .await;
    }

    /// The signing key for a state transition on the DPNS contract's `document_type_name`,
    /// with the contract fetched for that type so its absence (a network before the type
    /// existed) is reported before anything is signed.
    async fn select_dpns_signing_key_for(
        &self,
        identity_id: &Identifier,
        document_type_name: &str,
        signer: &impl Signer<IdentityPublicKey>,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let contract = self
            .system_contract(dpns_contract_id(), document_type_name)
            .await?;
        self.select_signing_key_for_document_type(
            identity_id,
            &contract,
            document_type_name,
            signer,
        )
        .await
    }
}
