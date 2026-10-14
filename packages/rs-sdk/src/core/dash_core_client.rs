//! Core RPC client used to retrieve quorum keys from core.
//!
//! TODO: This is a temporary implementation, effective until we integrate SPV
//! into dash-platform-sdk.

use crate::error::Error;
use dash_context_provider::ContextProviderError;
use dashcore_rpc::{
    dashcore::{hashes::Hash, Amount, QuorumHash},
    dashcore_rpc_json as json,
    json::{ProTxList, ProTxListType},
    jsonrpc::error::{Error as JsonRpcError, RpcError},
    jsonrpc::simple_http::Error as HttpError,
    Auth, Client, RpcApi,
};
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc;
use dpp::prelude::CoreBlockHeight;
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

    /// Return the wallet balance.
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
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
            .get_quorum_info_reversed(json::QuorumType::from(quorum_type), &quorum_hash, None)
            .map_err(|e: dashcore_rpc::Error| ContextProviderError::Generic(e.to_string()))?;
        let key = quorum_info.quorum_public_key;
        let pubkey = <Vec<u8> as TryInto<[u8; 48]>>::try_into(key).map_err(|_e| {
            ContextProviderError::InvalidQuorum(
                "quorum public key is not 48 bytes long".to_string(),
            )
        })?;
        Ok(pubkey)
    }

    /// Retrieve platform activation height from core.
    pub fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        let core = self.core.lock().expect("Core lock poisoned");

        let fork_info = platform_activation_fork(&*core)
            .map_err(|e: dashcore_rpc::Error| ContextProviderError::Generic(e.to_string()))?
            .ok_or(ContextProviderError::ActivationForkError(
                "no fork info for mn_rr".to_string(),
            ))?;

        fork_info
            .height
            .ok_or(ContextProviderError::ActivationForkError(
                "unknown fork height".to_string(),
            ))
    }

    /// Request list of Platform validators from Core.
    ///
    /// See also [Dash Core documentation](https://docs.dash.org/projects/core/en/stable/docs/api/remote-procedure-calls-evo.html#protx-list)
    #[allow(unused)]
    pub fn protx_list(
        &self,
        height: Option<u32>,
        protx_type: Option<ProTxListType>,
    ) -> Result<Vec<ProTxHash>, Error> {
        let core = self.core.lock().expect("Core lock poisoned");

        let pro_tx_hashes =
            core.get_protx_list(protx_type, Some(false), height)
                .map(|x| match x {
                    ProTxList::Hex(hex) => hex,
                    ProTxList::Info(info) => info.into_iter().map(|v| v.pro_tx_hash).collect(),
                })?;

        Ok(pro_tx_hashes)
    }
}

/// Core's JSON-RPC error code for a method it does not have.
const CORE_RPC_METHOD_NOT_FOUND: i32 = -32601;

/// The HTTP status Core refuses a request with, such as a method missing from the RPC user's
/// `rpcwhitelist`.
const HTTP_FORBIDDEN: u16 = 403;

/// Whether Core refused the request with HTTP 403 and no JSON-RPC answer.
fn is_http_forbidden(error: &dashcore_rpc::Error) -> bool {
    let dashcore_rpc::Error::JsonRpc(JsonRpcError::Transport(transport)) = error else {
        return false;
    };
    matches!(
        transport.downcast_ref::<HttpError>(),
        Some(HttpError::HttpErrorCode(HTTP_FORBIDDEN))
    )
}

/// Reads `mn_rr`, the Core deployment that activates Platform.
fn platform_activation_fork(
    core: &impl RpcApi,
) -> Result<Option<json::SoftforkInfo>, dashcore_rpc::Error> {
    match core.get_deployment_info(None) {
        Ok(deployment_info) => Ok(deployment_info.deployments.get("mn_rr").cloned()),
        // Core before v24 has no getdeploymentinfo, and reports the same deployments as
        // getblockchaininfo's softforks. Core v24 reports them there only with
        // -deprecatedrpc=softforks.
        Err(dashcore_rpc::Error::JsonRpc(JsonRpcError::Rpc(RpcError {
            code: CORE_RPC_METHOD_NOT_FOUND,
            ..
        }))) => core
            .get_blockchain_info()
            .map(|blockchain_info| blockchain_info.softforks.get("mn_rr").cloned()),
        // Core refuses a method missing from the RPC user's rpcwhitelist with HTTP 403. Core
        // before v24 still reports the deployment in softforks. Core v24 started without
        // -deprecatedrpc=softforks reports none there, so when softforks has no entry for
        // mn_rr, the refusal is returned rather than `None`.
        Err(e) if is_http_forbidden(&e) => {
            match core.get_blockchain_info()?.softforks.get("mn_rr") {
                Some(fork_info) => Ok(Some(fork_info.clone())),
                None => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::{platform_activation_fork, CORE_RPC_METHOD_NOT_FOUND};
    use dashcore_rpc::json::{SoftforkInfo, SoftforkType};
    use dashcore_rpc::jsonrpc::error::{Error as JsonRpcError, RpcError};
    use dashcore_rpc::jsonrpc::simple_http::Error as HttpError;
    use dashcore_rpc::RpcApi;
    use dpp::dashcore_rpc;
    use std::cell::RefCell;

    /// `getblockchaininfo` of Dash Core v24.0.0-rc.3 on regtest: no `softforks`, which Core
    /// v24 reports only with `-deprecatedrpc=softforks`.
    const CORE_V24_GETBLOCKCHAININFO: &str = r#"{
      "chain": "regtest", "blocks": 440, "headers": 440,
      "bestblockhash": "74b4fb2caeeebbc7cd5dcc50b58532d983f0e5c87f3ac729c6d47f0ace76b97d",
      "difficulty": 4.656542373906925e-10, "time": 1791945745, "mediantime": 1791945744,
      "verificationprogress": 1, "initialblockdownload": false,
      "chainwork": "0000000000000000000000000000000000000000000000000000000000000372",
      "size_on_disk": 98178, "pruned": false, "warnings": ""
    }"#;

    /// `getdeploymentinfo` of the same Core v24 node, its deployments cut down to `mn_rr`.
    const CORE_V24_GETDEPLOYMENTINFO: &str = r#"{
      "hash": "74b4fb2caeeebbc7cd5dcc50b58532d983f0e5c87f3ac729c6d47f0ace76b97d",
      "height": 440,
      "deployments": {"mn_rr": {"type": "buried", "active": true, "height": 432}}
    }"#;

    /// `getblockchaininfo` of Dash Core v23.1.8 on regtest, its softforks cut down to `mn_rr`.
    const CORE_V23_GETBLOCKCHAININFO: &str = r#"{
      "chain": "regtest", "blocks": 440, "headers": 440,
      "bestblockhash": "609b799530c36b8ad9d40b6c4dc182e9d64679bfe6c9252ed60822bbd8c8e748",
      "difficulty": 4.656542373906925e-10, "time": 1791945759, "mediantime": 1791945758,
      "verificationprogress": 1, "initialblockdownload": false,
      "chainwork": "0000000000000000000000000000000000000000000000000000000000000372",
      "size_on_disk": 98178, "pruned": false,
      "softforks": {"mn_rr": {"type": "buried", "active": true, "height": 432}},
      "warnings": ""
    }"#;

    /// How Core turns down `getdeploymentinfo`.
    #[derive(Clone, Copy)]
    enum Refusal {
        /// A JSON-RPC error with this code.
        Rpc(i32),
        /// This HTTP status and no body, as Core refuses a method missing from the RPC user's
        /// `rpcwhitelist` with 403.
        Http(u16),
    }

    impl Refusal {
        /// The error the JSON-RPC client returns for this answer.
        fn error(self) -> dashcore_rpc::Error {
            dashcore_rpc::Error::JsonRpc(match self {
                Refusal::Rpc(code) => JsonRpcError::Rpc(RpcError {
                    code,
                    message: String::new(),
                    data: None,
                }),
                Refusal::Http(status) => {
                    JsonRpcError::Transport(Box::new(HttpError::HttpErrorCode(status)))
                }
            })
        }
    }

    /// Answers like one Core version, and records the methods it was asked.
    struct FakeCore {
        /// The `getdeploymentinfo` result, or how Core turns it down.
        deployment_info: Result<&'static str, Refusal>,
        blockchain_info: &'static str,
        asked: RefCell<Vec<String>>,
    }

    impl FakeCore {
        fn new(
            deployment_info: Result<&'static str, Refusal>,
            blockchain_info: &'static str,
        ) -> Self {
            Self {
                deployment_info,
                blockchain_info,
                asked: RefCell::default(),
            }
        }
    }

    impl RpcApi for FakeCore {
        fn call<T: for<'a> serde::de::Deserialize<'a>>(
            &self,
            cmd: &str,
            _args: &[serde_json::Value],
        ) -> dashcore_rpc::Result<T> {
            self.asked.borrow_mut().push(cmd.to_string());
            let answer = match cmd {
                "getdeploymentinfo" => self.deployment_info.map_err(Refusal::error)?,
                "getblockchaininfo" => self.blockchain_info,
                _ => return Err(Refusal::Rpc(CORE_RPC_METHOD_NOT_FOUND).error()),
            };
            Ok(serde_json::from_str(answer)?)
        }
    }

    fn active_mn_rr() -> SoftforkInfo {
        SoftforkInfo {
            softfork_type: SoftforkType::Buried,
            active: true,
            height: Some(432),
            bip9: None,
        }
    }

    #[test]
    fn should_read_platform_activation_from_getdeploymentinfo_on_core_v24() {
        let core = FakeCore::new(Ok(CORE_V24_GETDEPLOYMENTINFO), CORE_V24_GETBLOCKCHAININFO);

        let fork = platform_activation_fork(&core).expect("Core v24 reports mn_rr");

        assert_eq!(fork, Some(active_mn_rr()));
        assert_eq!(*core.asked.borrow(), ["getdeploymentinfo"]);
    }

    #[test]
    fn should_read_platform_activation_from_softforks_on_core_v23() {
        let core = FakeCore::new(
            Err(Refusal::Rpc(CORE_RPC_METHOD_NOT_FOUND)),
            CORE_V23_GETBLOCKCHAININFO,
        );

        let fork = platform_activation_fork(&core).expect("Core v23 reports mn_rr");

        assert_eq!(fork, Some(active_mn_rr()));
        assert_eq!(
            *core.asked.borrow(),
            ["getdeploymentinfo", "getblockchaininfo"]
        );
    }

    #[test]
    fn should_not_read_softforks_when_getdeploymentinfo_fails_for_another_reason() {
        let core = FakeCore::new(Err(Refusal::Rpc(-1)), CORE_V23_GETBLOCKCHAININFO);

        let fork = platform_activation_fork(&core);

        assert!(
            matches!(
                fork,
                Err(dashcore_rpc::Error::JsonRpc(JsonRpcError::Rpc(RpcError {
                    code: -1,
                    ..
                })))
            ),
            "expected the getdeploymentinfo error, got {fork:?}"
        );
        assert_eq!(*core.asked.borrow(), ["getdeploymentinfo"]);
    }

    #[test]
    fn should_read_platform_activation_from_softforks_when_core_v23_refuses_getdeploymentinfo() {
        let core = FakeCore::new(Err(Refusal::Http(403)), CORE_V23_GETBLOCKCHAININFO);

        let fork = platform_activation_fork(&core).expect("Core v23 reports mn_rr");

        assert_eq!(fork, Some(active_mn_rr()));
        assert_eq!(
            *core.asked.borrow(),
            ["getdeploymentinfo", "getblockchaininfo"]
        );
    }

    #[test]
    fn should_fail_with_the_refusal_when_core_v24_refuses_getdeploymentinfo() {
        let core = FakeCore::new(Err(Refusal::Http(403)), CORE_V24_GETBLOCKCHAININFO);

        let fork = platform_activation_fork(&core);

        // Core v24 reports no softforks, so their absence must not read as Platform not being
        // activated.
        assert!(
            matches!(
                &fork,
                Err(dashcore_rpc::Error::JsonRpc(JsonRpcError::Transport(transport)))
                    if matches!(
                        transport.downcast_ref::<HttpError>(),
                        Some(HttpError::HttpErrorCode(403))
                    )
            ),
            "expected the getdeploymentinfo refusal, got {fork:?}"
        );
        assert_eq!(
            *core.asked.borrow(),
            ["getdeploymentinfo", "getblockchaininfo"]
        );
    }

    #[test]
    fn should_not_read_softforks_when_core_answers_getdeploymentinfo_with_another_http_status() {
        let core = FakeCore::new(Err(Refusal::Http(401)), CORE_V23_GETBLOCKCHAININFO);

        let fork = platform_activation_fork(&core);

        assert!(
            matches!(
                &fork,
                Err(dashcore_rpc::Error::JsonRpc(JsonRpcError::Transport(transport)))
                    if matches!(
                        transport.downcast_ref::<HttpError>(),
                        Some(HttpError::HttpErrorCode(401))
                    )
            ),
            "expected the getdeploymentinfo error, got {fork:?}"
        );
        assert_eq!(*core.asked.borrow(), ["getdeploymentinfo"]);
    }

    #[test]
    fn should_report_no_activation_when_core_v24_does_not_report_mn_rr() {
        let core = FakeCore::new(
            Ok(r#"{
              "hash": "74b4fb2caeeebbc7cd5dcc50b58532d983f0e5c87f3ac729c6d47f0ace76b97d",
              "height": 440,
              "deployments": {}
            }"#),
            CORE_V23_GETBLOCKCHAININFO,
        );

        let fork = platform_activation_fork(&core).expect("Core v24 answers");

        assert_eq!(fork, None);
        assert_eq!(*core.asked.borrow(), ["getdeploymentinfo"]);
    }
}
