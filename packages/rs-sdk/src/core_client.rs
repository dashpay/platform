//! Core RPC client used to retrieve quorum keys from core.
//!
//! TODO: This is a temporary implementation, effective until we integrate SPV
//! into dash-platform-sdk.

use dashcore_rpc::{
    dashcore::{hashes::Hash, Amount, QuorumHash},
    dashcore_rpc_json as json, Auth, Client, RpcApi,
};
use drive_proof_verifier::error::ContextProviderError;
use std::{fmt::Debug, sync::Mutex};

use crate::error::Error;

/// Core RPC client that can be used to retrieve quorum keys from core.
///
/// Implements [`ContextProvider`] trait.
///
/// TODO: This is a temporary implementation, effective until we integrate SPV.
pub struct CoreClient {
    core: Mutex<Client>,
    server_address: String,
    core_user: String,
    core_port: u16,
}

impl Debug for CoreClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreClient")
            .field("server_address", &self.server_address)
            .field("core_user", &self.core_user)
            .field("core_port", &self.core_port)
            .finish()
    }
}

impl CoreClient {
    /// Create new Dash Core client.
    ///
    /// # Arguments
    ///
    /// * `server_address` - Dash Core server address.
    /// * `core_port` - Dash Core port.
    /// * `core_user` - Dash Core user.
    /// * `core_password` - Dash Core password.
    pub fn new(
        server_address: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
    ) -> Result<Self, Error> {
        let addr = format!("http://{}:{}", server_address, core_port);
        let core = Client::new(
            &addr,
            Auth::UserPass(core_user.to_string(), core_password.to_string()),
        )
        .map_err(Error::CoreClientError)?;

        Ok(Self {
            core: Mutex::new(core),
            server_address: server_address.to_string(),
            core_user: core_user.to_string(),
            core_port,
        })
    }
}

// Wallet functions
impl CoreClient {
    /// List unspent transactions
    ///
    /// ## Arguments
    ///
    /// * `minimum_sum_satoshi` - Minimum total sum of all returned unspent transactions
    ///
    /// ## See also
    ///
    /// * [Dash Core documentation](https://docs.dash.org/projects/core/en/stable/docs/api/remote-procedure-calls-wallet.html#listunspent)
    pub fn list_unspent(
        &self,
        minimum_sum_satoshi: Option<u64>,
    ) -> Result<Vec<dashcore_rpc::json::ListUnspentResultEntry>, Error> {
        let options = json::ListUnspentQueryOptions {
            minimum_sum_amount: minimum_sum_satoshi.map(Amount::from_sat),
            ..Default::default()
        };

        self.core
            .lock()
            .expect("Core lock poisoned")
            .list_unspent(None, None, None, None, Some(options))
            .map_err(Error::CoreClientError)
    }

    /// Return address to which change of transaction can be sent.
    pub fn get_balance(&self) -> Result<Amount, Error> {
        self.core
            .lock()
            .expect("Core lock poisoned")
            .get_balance(None, None)
            .map_err(Error::CoreClientError)
    }

    /// Retrieve quorum public key from core.
    pub fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
    ) -> Result<[u8; 48], ContextProviderError> {
        let quorum_hash = QuorumHash::from_slice(&quorum_hash)
            .map_err(|e| ContextProviderError::InvalidQuorum(e.to_string()))?;

        let core = self.core.lock().expect("Core lock poisoned");
        let quorum_info = core
            .get_quorum_info(json::QuorumType::from(quorum_type), &quorum_hash, None)
            .map_err(|e: dashcore_rpc::Error| ContextProviderError::InvalidQuorum(e.to_string()))?;
        let key = quorum_info.quorum_public_key;
        let pubkey = <Vec<u8> as TryInto<[u8; 48]>>::try_into(key).map_err(|_e| {
            ContextProviderError::InvalidQuorum(
                "quorum public key is not 48 bytes long".to_string(),
            )
        })?;
        Ok(pubkey)
    }
}
