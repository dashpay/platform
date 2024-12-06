//! Core RPC client used to retrieve quorum keys from core.
//!
//! TODO: This is a temporary implementation, effective until we integrate SPV
//! into dash-platform-sdk.

use crate::error::Error;
use dashcore_rpc::{
    dashcore::{hashes::Hash, Amount, QuorumHash},
    dashcore_rpc_json as json,
    json::{ProTxList, ProTxListType},
    Auth, Client, RpcApi,
};
use dpp::dashcore::ProTxHash;
use dpp::prelude::CoreBlockHeight;
use drive_proof_verifier::error::ContextProviderError;
use std::time::Duration;
use std::{fmt::Debug, sync::Mutex};
use zeroize::Zeroizing;

/// Core RPC client that can be used to retrieve quorum keys from core.
///
/// TODO: This is a temporary implementation, effective until we integrate SPV.
pub struct LowLevelDashCoreClient {
    core: Mutex<Client>,
    server_address: String,
    core_user: String,
    core_password: Zeroizing<String>,
    core_port: u16,
}

/// Client still warming up
pub const CORE_RPC_ERROR_IN_WARMUP: i32 = -28;
/// Dash is not connected
pub const CORE_RPC_CLIENT_NOT_CONNECTED: i32 = -9;
/// Still downloading initial blocks
pub const CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD: i32 = -10;

macro_rules! retry {
    ($action:expr) => {{
        /// Maximum number of retry attempts
        const MAX_RETRIES: u32 = 4;
        /// // Multiplier for Fibonacci sequence
        const FIB_MULTIPLIER: u64 = 1;

        const BASE_TIME_MS: u64 = 40;

        fn fibonacci(n: u32) -> u64 {
            match n {
                0 => 0,
                1 => 1,
                _ => fibonacci(n - 1) + fibonacci(n - 2),
            }
        }

        let mut final_result = None;
        for i in 0..MAX_RETRIES {
            match $action {
                Ok(result) => {
                    final_result = Some(Ok(result));
                    break;
                }
                Err(e) => {
                    match e {
                        dashcore_rpc::Error::JsonRpc(
                            // Retry on transport connection error
                            dashcore_rpc::jsonrpc::error::Error::Transport(_)
                            | dashcore_rpc::jsonrpc::error::Error::Rpc(
                                // Retry on Core RPC "not ready" errors
                                dashcore_rpc::jsonrpc::error::RpcError {
                                    code:
                                        CORE_RPC_ERROR_IN_WARMUP
                                        | CORE_RPC_CLIENT_NOT_CONNECTED
                                        | CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
                                    ..
                                },
                            ),
                        ) => {
                            if i == MAX_RETRIES - 1 {
                                final_result =
                                    Some(Err(ContextProviderError::Generic(e.to_string())));
                            }
                            let delay = fibonacci(i + 2) * FIB_MULTIPLIER;
                            std::thread::sleep(Duration::from_millis(delay * BASE_TIME_MS));
                        }
                        _ => return Err(ContextProviderError::Generic(e.to_string())),
                    };
                }
            }
        }
        final_result.expect("expected a final result")
    }};
}

impl Clone for LowLevelDashCoreClient {
    // As Client does not implement Clone, we just create a new instance of CoreClient here.
    fn clone(&self) -> Self {
        LowLevelDashCoreClient::new(
            &self.server_address,
            self.core_port,
            &self.core_user,
            &self.core_password,
        )
        .expect("Failed to clone CoreClient when cloning, this should not happen")
    }
}

impl Debug for LowLevelDashCoreClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreClient")
            .field("server_address", &self.server_address)
            .field("core_user", &self.core_user)
            .field("core_port", &self.core_port)
            .finish()
    }
}

impl LowLevelDashCoreClient {
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
            core_password: core_password.to_string().into(),
            core_port,
        })
    }
}

// Wallet functions
impl LowLevelDashCoreClient {
    /// List unspent transactions
    ///
    /// ## Arguments
    ///
    /// * `minimum_sum_satoshi` - Minimum total sum of all returned unspent transactions
    ///
    /// ## See also
    ///
    /// * [Dash Core documentation](https://docs.dash.org/projects/core/en/stable/docs/api/remote-procedure-calls-wallet.html#listunspent)
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    pub fn list_unspent(
        &self,
        minimum_sum_satoshi: Option<u64>,
    ) -> Result<Vec<dashcore_rpc::json::ListUnspentResultEntry>, ContextProviderError> {
        let options = json::ListUnspentQueryOptions {
            minimum_sum_amount: minimum_sum_satoshi.map(Amount::from_sat),
            ..Default::default()
        };

        let core = self.core.lock().expect("Core lock poisoned");

        retry!(core.list_unspent(None, None, None, None, Some(options.clone())))
    }

    /// Return address to which change of transaction can be sent.
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    pub fn get_balance(&self) -> Result<Amount, ContextProviderError> {
        let core = self.core.lock().expect("Core lock poisoned");
        retry!(core.get_balance(None, None))
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

        // Retrieve the quorum info
        let quorum_info: json::QuorumInfoResult =
            retry!(core.get_quorum_info(json::QuorumType::from(quorum_type), &quorum_hash, None))?;

        // Extract the quorum public key and attempt to convert it
        let key = quorum_info.quorum_public_key;
        let pubkey = <Vec<u8> as TryInto<[u8; 48]>>::try_into(key).map_err(|_| {
            ContextProviderError::InvalidQuorum(
                "quorum public key is not 48 bytes long".to_string(),
            )
        })?;

        Ok(pubkey)
    }

    /// Retrieve platform activation height from core.
    pub fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        let core = self.core.lock().expect("Core lock poisoned");

        let blockchain_info = retry!(core.get_blockchain_info())?;

        let fork_info = blockchain_info.softforks.get("mn_rr").ok_or(
            ContextProviderError::ActivationForkError("no fork info for mn_rr".to_string()),
        )?;

        fork_info
            .height
            .ok_or(ContextProviderError::ActivationForkError(
                "unknown fork height".to_string(),
            ))
    }

    /// Require list of validators from Core.
    ///
    /// See also [Dash Core documentation](https://docs.dash.org/projects/core/en/stable/docs/api/remote-procedure-calls-evo.html#protx-list)
    #[allow(unused)]
    pub fn protx_list(
        &self,
        height: Option<u32>,
        protx_type: Option<ProTxListType>,
    ) -> Result<Vec<ProTxHash>, ContextProviderError> {
        let core = self.core.lock().expect("Core lock poisoned");

        let pro_tx_list = retry!(core.get_protx_list(protx_type.clone(), Some(false), height))?;
        let pro_tx_hashes = match pro_tx_list {
            ProTxList::Hex(hex) => hex,
            ProTxList::Info(info) => info.into_iter().map(|v| v.pro_tx_hash).collect(),
        };

        Ok(pro_tx_hashes)
    }
}
