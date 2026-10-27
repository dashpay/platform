//! Shared lookups for token state-transition methods on
//! [`IdentityWallet`]. All token actions need to (a) fetch the data
//! contract by id and (b) resolve the canonical AUTHENTICATION /
//! CRITICAL / ECDSA_SECP256K1 signing key on the actor identity,
//! so those steps live here rather than being copy-pasted across each
//! action file.
//!
//! `CRITICAL` is non-negotiable: rs-dpp's
//! `combined_security_level_requirement` collapses the allowed key
//! set for *any* batch containing a token transition to
//! `[SecurityLevel::CRITICAL]` only — MASTER, HIGH, and MEDIUM are
//! all rejected by Drive. See
//! `state_transitions/document/batch_transition/methods/v0/mod.rs:133-138`.

use super::super::key_selection::{
    check_signing_key, usable_signing_key, SigningScope, PLATFORM_SIGNING_KEY_TYPES,
    WALLET_SIGNING_KEY_TYPES,
};
use std::sync::Arc;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

/// The security levels [`token_signing_key`] accepts: consensus takes only CRITICAL for a
/// batch with a token transition.
const TOKEN_SIGNING_LEVELS: [SecurityLevel; 1] = [SecurityLevel::CRITICAL];

/// The key that signs an operation on the token at `position` of `token_contract_id`: a
/// CRITICAL AUTHENTICATION ECDSA key whose bounds cover that token.
fn token_signing_key<'a>(
    identity: &'a Identity,
    token_contract_id: Identifier,
    position: TokenContractPosition,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key(
        identity,
        signer,
        Purpose::AUTHENTICATION,
        &TOKEN_SIGNING_LEVELS,
        &WALLET_SIGNING_KEY_TYPES,
        SigningScope::Token {
            contract_id: token_contract_id,
            position,
        },
    )
}

/// The purposes a key may have to sign a token operation.
pub(super) const TOKEN_SIGNING_PURPOSES: [Purpose; 1] = [Purpose::AUTHENTICATION];

/// The purposes a key may have to sign a batch of exactly one token transfer or token claim:
/// consensus also takes a TRANSFER key there (`BatchTransitionV1::purpose_requirement`).
pub(super) const LONE_TOKEN_TRANSFER_SIGNING_PURPOSES: [Purpose; 2] =
    [Purpose::AUTHENTICATION, Purpose::TRANSFER];

/// Check a key the caller chose to sign an operation on the token at `position` of
/// `data_contract`, with one of
/// `purposes`, by the rules [`token_signing_key`] applies except that any key type Platform
/// verifies is accepted, so a key Platform would refuse fails before any request. Contract
/// bounds limit only an AUTHENTICATION key.
pub(super) fn check_token_signing_key(
    key: &IdentityPublicKey,
    signer: &impl Signer<IdentityPublicKey>,
    data_contract: &DataContract,
    position: TokenContractPosition,
    purposes: &[Purpose],
) -> Result<(), dash_sdk::Error> {
    check_signing_key(
        key,
        signer,
        purposes,
        &TOKEN_SIGNING_LEVELS,
        &PLATFORM_SIGNING_KEY_TYPES,
        SigningScope::Token {
            contract_id: data_contract.id(),
            position,
        },
    )
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Resolve the AUTHENTICATION signing key for token state
    /// transitions — used by the `_with_external_signer` variants
    /// where the caller brings the `Signer` and we just need to pick
    /// which on-chain key id to sign with.
    ///
    /// Key selection is a fixed canonical policy:
    /// `Purpose::AUTHENTICATION` × `SecurityLevel::CRITICAL` × `ECDSA_SECP256K1`,
    /// bounds covering `token_contract_id` (see `token_signing_key`).
    /// Callers that pass a `signing_key_id` over FFI are advisory —
    /// this Rust path is authoritative and Drive validates the chosen
    /// key matches what the state transition needs.
    pub(super) async fn token_resolve_signing_key(
        &self,
        identity_id: &Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        signer: &impl Signer<IdentityPublicKey>,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        let identity = info
            .identity_manager
            .identity(identity_id)
            .map(|m| m.identity.clone())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        drop(wm);

        let signing_key = token_signing_key(&identity, token_contract_id, token_position, signer)?
            .ok_or_else(|| PlatformWalletError::NoUsableSigningKey {
                identity_id: *identity_id,
                purpose: Purpose::AUTHENTICATION,
                security_levels: TOKEN_SIGNING_LEVELS.to_vec(),
                key_types: WALLET_SIGNING_KEY_TYPES.to_vec(),
                transition: format!("a token transition of contract {token_contract_id}"),
            })?
            .clone();

        Ok(signing_key)
    }

    /// Fetch a data contract by id from Platform.
    pub(super) async fn token_fetch_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Arc<DataContract>, PlatformWalletError> {
        use dash_sdk::platform::{ContextProvider, Fetch};

        let contract = DataContract::fetch(&self.sdk, contract_id)
            .await
            .map_err(|e| PlatformWalletError::TokenError(format!("Fetch contract failed: {}", e)))?
            .ok_or_else(|| {
                PlatformWalletError::TokenError(format!(
                    "Data contract {} not found on Platform",
                    contract_id
                ))
            })?;
        let contract = Arc::new(contract);
        // Register the fetched contract with the SDK context provider so the
        // returned-proof verification after a token state-transition broadcast
        // can resolve it. Mirrors document.rs's
        // `register_contract_for_proof_verification`, done here at the shared
        // token fetch point so every write path (mint / burn / transfer /
        // purchase / freeze / …) is covered. The mobile
        // `TrustedHttpContextProvider` never fetches contracts itself, so
        // without this the returned-proof verification fails with "unknown
        // contract" even though the token write already landed on-chain.
        if let Some(provider) = self.sdk.context_provider() {
            provider.register_data_contract(Arc::clone(&contract));
        }
        Ok(contract)
    }
}

#[cfg(test)]
mod signing_tests {
    use super::*;
    use crate::wallet::identity::network::key_selection::tests::{
        bound_key, key, refused_purposes,
    };
    use crate::wallet::identity::network::signing_key::tests::{
        assert_no_usable_key, identity as identity_with_keys, lock_checking_signer,
        wallet_with_identity, wallet_with_signing_keys, KeyFilter,
    };
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::KeyType;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;

    /// A contract to point token operations at; the key check runs before its tokens matter.
    fn some_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("bundled DPNS contract"),
        )
    }

    fn transfer_key(security_level: SecurityLevel) -> IdentityPublicKey {
        let mut key = key(1, security_level);
        key.set_purpose(Purpose::TRANSFER);
        key
    }

    #[tokio::test]
    async fn should_accept_an_explicit_transfer_key_for_a_lone_token_transfer_or_claim() {
        let wallet = wallet_with_signing_keys().await;
        let identity = wallet.identity();
        let signer = KeyFilter(|_| true);
        let owner = Identifier::from([1; 32]);
        let critical = transfer_key(SecurityLevel::CRITICAL);
        let transfer = identity
            .token_transfer_with_signer(
                some_contract(),
                0,
                owner,
                Identifier::from([2; 32]),
                1,
                &critical,
                &signer,
                None,
                None,
            )
            .await
            .err()
            .expect("the mock SDK has no answers");
        assert!(
            matches!(transfer, dash_sdk::Error::DapiClientError(_)),
            "a CRITICAL TRANSFER key may sign a token transfer, so the request goes out: \
             {transfer:?}"
        );
        let claim = identity
            .token_claim_with_signer(
                some_contract(),
                0,
                owner,
                TokenDistributionType::Perpetual,
                &critical,
                &signer,
                None,
                None,
            )
            .await
            .err()
            .expect("the mock SDK has no answers");
        assert!(
            matches!(claim, dash_sdk::Error::DapiClientError(_)),
            "a CRITICAL TRANSFER key may sign a token claim, so the request goes out: {claim:?}"
        );
        let high = transfer_key(SecurityLevel::HIGH);
        let transfer = identity
            .token_transfer_with_signer(
                some_contract(),
                0,
                owner,
                Identifier::from([2; 32]),
                1,
                &high,
                &signer,
                None,
                None,
            )
            .await
            .err()
            .expect("the key must be refused");
        assert!(
            matches!(
                transfer,
                dash_sdk::Error::Protocol(
                    ProtocolError::InvalidSignaturePublicKeySecurityLevelError(_)
                )
            ),
            "a TRANSFER key signs a batch only at CRITICAL: {transfer:?}"
        );
        let burn = identity
            .token_burn_with_signer(
                some_contract(),
                0,
                owner,
                1,
                &critical,
                &signer,
                None,
                None,
                None,
            )
            .await
            .err()
            .expect("the key must be refused");
        assert_eq!(
            refused_purposes(&burn),
            Some(vec![Purpose::AUTHENTICATION]),
            "only a lone transfer or claim takes a TRANSFER key: {burn:?}"
        );
    }

    #[tokio::test]
    async fn should_accept_an_explicit_token_key_of_any_type_platform_verifies() {
        let wallet = wallet_with_signing_keys().await;
        let mut hash160 = key(1, SecurityLevel::CRITICAL);
        hash160.set_key_type(KeyType::ECDSA_HASH160);
        let burn = wallet
            .identity()
            .token_burn_with_signer(
                some_contract(),
                0,
                Identifier::from([1; 32]),
                1,
                &hash160,
                &KeyFilter(|_| true),
                None,
                None,
                None,
            )
            .await
            .err()
            .expect("the mock SDK has no answers");
        assert!(
            matches!(burn, dash_sdk::Error::DapiClientError(_)),
            "Platform verifies an ECDSA_HASH160 signature, so the request goes out: {burn:?}"
        );
    }

    #[tokio::test]
    async fn should_refuse_an_explicit_token_key_before_any_network_call() {
        let wallet = wallet_with_signing_keys().await;
        let contract = some_contract();
        let burn = |signing_key: IdentityPublicKey| {
            let wallet = &wallet;
            let contract = Arc::clone(&contract);
            async move {
                wallet
                    .identity()
                    .token_burn_with_signer(
                        contract,
                        0,
                        Identifier::from([1; 32]),
                        1,
                        &signing_key,
                        &KeyFilter(|_| true),
                        None,
                        None,
                        None,
                    )
                    .await
                    .err()
                    .expect("the key must be refused")
            }
        };
        let error = burn(key(1, SecurityLevel::HIGH)).await;
        assert!(
            matches!(
                error,
                dash_sdk::Error::Protocol(
                    ProtocolError::InvalidSignaturePublicKeySecurityLevelError(_)
                )
            ),
            "a token transition takes only a CRITICAL key: {error:?}"
        );
        let bound_to_one_document_type = bound_key(
            1,
            SecurityLevel::CRITICAL,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract.id(),
                document_type_name: "domain".to_string(),
            }),
        );
        let error = burn(bound_to_one_document_type).await;
        assert!(
            matches!(
                &error,
                dash_sdk::Error::Protocol(ProtocolError::ConsensusError(consensus))
                    if matches!(
                        consensus.as_ref(),
                        ConsensusError::SignatureError(
                            SignatureError::ContractBoundedKeyOutOfBoundsError(_)
                        )
                    )
            ),
            "a token transition needs bounds covering the whole contract: {error:?}"
        );
    }

    #[tokio::test]
    async fn should_report_no_usable_token_key_as_a_typed_error() {
        let identity = identity_with_keys(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
        );
        let identity_id = identity.id();
        let wallet = wallet_with_identity(identity).await;
        let error = wallet
            .identity()
            .token_resolve_signing_key(&identity_id, Identifier::default(), 0, &KeyFilter(|_| true))
            .await
            .unwrap_err();
        assert_no_usable_key(error, identity_id, SecurityLevel::CRITICAL);
    }

    #[tokio::test]
    async fn should_check_token_key_availability_outside_wallet_lock() {
        let wallet = wallet_with_signing_keys().await;
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .token_resolve_signing_key(&Identifier::default(), Identifier::default(), 0, &signer)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("available to signer"), "{error}");
    }

    #[test]
    fn should_sign_a_token_operation_with_a_key_that_covers_the_whole_contract() {
        use crate::wallet::identity::network::key_selection::tests::{bound_key, identity, key};
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;

        let token_contract = Identifier::from([0x70; 32]);
        let identity = identity(vec![
            bound_key(
                1,
                SecurityLevel::CRITICAL,
                Some(ContractBounds::SingleContractDocumentType {
                    id: token_contract,
                    document_type_name: "note".to_string(),
                }),
            ),
            key(2, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(3, SecurityLevel::CRITICAL),
        ]);
        let chosen = token_signing_key(&identity, token_contract, 0, &KeyFilter(|_| true)).unwrap();
        assert_eq!(
            chosen.map(|key| key.id()),
            Some(3),
            "a document-type bound key cannot sign a token operation, and an unlimited key \
             is taken before a budgeted one"
        );
    }
}
