//! TODO: Not sure if it's needed at all
use dpp::{bls_signatures::PrivateKey, prelude::AssetLockProof};

use tokio_util::sync::CancellationToken;

use crate::wallet::Wallet;

#[derive(Clone)]
/// Various context information required for transition execution.
// TODO: Remove?
pub enum TransitionContext {
    /// Create identity with provided amount in Dash
    CreateIdentity { amount: u64 },
}
pub struct TransitionContext1<'a> {
    /// Cancellation token for transition execution.
    ///
    /// If this token is cancelled, transition execution will be aborted.
    pub cancellation_token: CancellationToken,
    /// Asset lock proof.
    ///
    /// Required to create an identity.
    pub asset_lock_proof: Option<AssetLockProof>,

    /// Asset lock private key.
    ///
    /// Required to create an identity.
    pub asset_lock_private_key: Option<PrivateKey>,

    /// Wallet to use for transition execution.
    ///
    /// If not provided, defaults from [Sdk] will be used.
    pub wallet: &'a dyn Wallet,
}

// impl<'a> TransitionContext<'a> {
//     /// Creates a new [TransitionContext], using defaults from provided [Sdk].
//     pub fn new(sdk: &'a Sdk) -> Self {
//         Self {
//             cancellation_token: CancellationToken::new(),
//             asset_lock_proof: None,
//             asset_lock_private_key: None,
//             wallet: sdk.wallet.as_ref().expect("Wallet not set in Sdk").as_ref(),
//         }
//     }

//     /// Provide instance of wallet to use for transition execution.
//     pub fn with_wallet(&mut self, wallet: &'a dyn Wallet) -> &mut Self {
//         self.wallet = wallet;
//         self
//     }

//     /// Locks some assets (funds) on Dash Core using the wallet.
//     ///
//     /// This is a conveniance method that calls [Wallet::lock_assets] and stores the returned proof and private key.
//     ///
//     /// If self.asset_lock_proof is already set and the amount is the same or higher, this method will return existing
//     /// proof and key.
//     pub async fn lock_assets(&mut self, amount: u64) -> Result<(), Error> {
//         if let Some(proof) = &self.asset_lock_proof {
//             if let Some(tx) = proof.transaction() {
//                 if let Some(TransactionPayload::AssetLockPayloadType(ref payload)) =
//                     tx.special_transaction_payload
//                 {
//                     if amount <= payload.credit_outputs.iter().map(|out| out.value).sum() {
//                         return Ok(());
//                     }
//                 }
//             }
//         }

//         let assets = self.wallet.lock_assets(amount).await?;
//         self.asset_lock_proof = Some(assets.0);
//         self.asset_lock_private_key = Some(assets.1);

//         Ok(())
//     }
// }

// #[derive(Debug)]
// pub struct TransitionContextBuilder<'a> {
//     stub_transition_context: TransitionContext<'a>,
// }

// impl<'a> TransitionContextBuilder<'a> {
//     pub fn new(sdk: &'a Sdk) -> Self {
//         Self {
//             stub_transition_context: TransitionContext {
//                 cancellation_token: CancellationToken::new(),
//                 asset_lock_proof_and_key: None,
//                 sdk,
//             },
//         }
//     }

//     pub fn with_asset_lock_proof(
//         &mut self,
//         asset_lock_proof: AssetLockProof,
//         asset_lock_one_time_private_key: PrivateKey,
//     ) -> &mut Self {
//         self.stub_transition_context.asset_lock_proof_and_key =
//             Some((asset_lock_proof, asset_lock_one_time_private_key));

//         self
//     }

//     pub fn build(self) -> TransitionContext<'a> {
//         self.stub_transition_context
//     }
// }
