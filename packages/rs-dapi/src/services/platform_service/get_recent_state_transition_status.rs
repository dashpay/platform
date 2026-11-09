//! `getRecentStateTransitionStatus`: this node's own view of a state transition, by hash.
//!
//! The answer comes from this node's Tenderdash at once, without waiting and without
//! re-broadcasting anything: committed (its transaction index has the hash), pending (its
//! mempool has it) or unknown. None of it is proved, and mempools differ between nodes, so a
//! client uses it only to decide whether waiting longer makes sense. Whether a transition can
//! still execute is a question for proved reads of its inputs.

use crate::error::DapiError;
use crate::services::platform_service::error_mapping::decode_consensus_error;
use crate::services::platform_service::wait_for_state_transition_result::validate_state_transition_hash;
use crate::services::platform_service::{PlatformServiceImpl, TenderdashStatus};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use dapi_grpc::platform::v0::get_recent_state_transition_status_response::get_recent_state_transition_status_response_v0::{
    Committed, Pending, Status as TransitionStatus, Unknown,
};
use dapi_grpc::platform::v0::get_recent_state_transition_status_response::{
    GetRecentStateTransitionStatusResponseV0, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::get_status_request::{GetStatusRequestV0, Version as StatusVersion};
use dapi_grpc::platform::v0::get_status_response::Version as StatusResponseVersion;
use dapi_grpc::platform::v0::{
    GetRecentStateTransitionStatusRequest, GetRecentStateTransitionStatusResponse,
    GetStatusRequest, ResponseMetadata, get_recent_state_transition_status_request,
};
use dapi_grpc::tonic::Request;
use tracing::debug;

use crate::clients::tenderdash_client::TxResponse;

impl PlatformServiceImpl {
    /// Report what this node knows of a state transition, by hash, without waiting.
    ///
    /// Reads the transaction index first, then the mempool, then the transaction index again:
    /// a transition that leaves the mempool for a block between the first two reads is found
    /// by the third, so it is not reported as unknown.
    pub async fn get_recent_state_transition_status_impl(
        &self,
        request: Request<GetRecentStateTransitionStatusRequest>,
    ) -> Result<GetRecentStateTransitionStatusResponse, DapiError> {
        let v0 = match request.into_inner().version {
            Some(get_recent_state_transition_status_request::Version::V0(v0)) => v0,
            None => {
                return Err(DapiError::InvalidArgument(
                    "get_recent_state_transition_status request must have v0".to_string(),
                ));
            }
        };
        validate_state_transition_hash(&v0.state_transition_hash)?;
        let hash_base64 = BASE64_STANDARD.encode(&v0.state_transition_hash);

        let status = self.recent_state_transition_status(&hash_base64).await?;
        let metadata = self.node_view_metadata().await;

        Ok(GetRecentStateTransitionStatusResponse {
            version: Some(ResponseVersion::V0(
                GetRecentStateTransitionStatusResponseV0 {
                    status: Some(status),
                    metadata,
                },
            )),
        })
    }

    async fn recent_state_transition_status(
        &self,
        hash_base64: &str,
    ) -> Result<TransitionStatus, DapiError> {
        if let Some(committed) = self.committed_state_transition(hash_base64).await? {
            return Ok(committed);
        }
        match self.tenderdash_client.unconfirmed_tx(hash_base64).await {
            Ok(_) => return Ok(TransitionStatus::Pending(Pending {})),
            Err(error) if is_not_found(&error) => {}
            Err(error) => return Err(error),
        }
        // It may have left the mempool for a block since the first read.
        if let Some(committed) = self.committed_state_transition(hash_base64).await? {
            return Ok(committed);
        }
        Ok(TransitionStatus::Unknown(Unknown {}))
    }

    /// The transition as this node's transaction index holds it, or `None` when it does not.
    async fn committed_state_transition(
        &self,
        hash_base64: &str,
    ) -> Result<Option<TransitionStatus>, DapiError> {
        match self.tenderdash_client.tx(hash_base64.to_string()).await {
            Ok(tx) => Ok(committed_status(tx)),
            Err(error) if is_not_found(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// The node's latest block, from the status `getStatus` serves (cached for a few seconds),
    /// so the client can tell how recent the answer is. Unproved, like the answer itself; a
    /// node whose status cannot be read answers without it.
    async fn node_view_metadata(&self) -> Option<ResponseMetadata> {
        let request = Request::new(GetStatusRequest {
            version: Some(StatusVersion::V0(GetStatusRequestV0 {})),
        });
        let status = match self.get_status_impl(request).await {
            Ok(response) => response.into_inner(),
            Err(error) => {
                debug!(%error, "get_recent_state_transition_status: node status unavailable");
                return None;
            }
        };
        let StatusResponseVersion::V0(status) = status.version?;
        let chain = status.chain.unwrap_or_default();
        let time = status.time.unwrap_or_default();
        let protocol_version = status
            .version
            .and_then(|version| version.protocol)
            .and_then(|protocol| protocol.drive)
            .map(|drive| drive.current)
            .unwrap_or_default();
        Some(ResponseMetadata {
            height: chain.latest_block_height,
            core_chain_locked_height: chain.core_chain_locked_height.unwrap_or_default(),
            epoch: time.epoch.unwrap_or_default(),
            time_ms: time.block.unwrap_or_default(),
            protocol_version,
            chain_id: status
                .network
                .map(|network| network.chain_id)
                .unwrap_or_default(),
        })
    }
}

/// What a transaction-index answer says: committed at its height, with its error when its code
/// is not 0, as `waitForStateTransitionResult` reports it. An answer with neither the
/// transaction nor its result is no answer (`handle_duplicate_transaction` reads it the same
/// way).
fn committed_status(tx: TxResponse) -> Option<TransitionStatus> {
    if tx.tx_result.is_empty() && tx.tx.is_empty() {
        return None;
    }
    let result = tx.tx_result;
    let error = (result.code != 0).then(|| {
        TenderdashStatus::new(
            i64::from(result.code),
            (!result.data.is_empty()).then_some(result.data),
            if result.info.is_empty() {
                None
            } else {
                decode_consensus_error(result.info)
            },
        )
        .into()
    });
    Some(TransitionStatus::Committed(Committed {
        height: u64::try_from(tx.height).unwrap_or_default(),
        error,
    }))
}

/// Whether a Tenderdash lookup failed because it does not have the hash, as opposed to failing.
fn is_not_found(error: &DapiError) -> bool {
    match error {
        DapiError::NotFound(_) => true,
        DapiError::TenderdashClientError(status) => status
            .message
            .as_deref()
            .is_some_and(|message| message.contains("not found")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::ExecTxResult;

    #[test]
    fn should_report_a_transaction_with_code_zero_as_committed_without_an_error() {
        let tx = TxResponse {
            height: 1234,
            tx: "AAEC".to_string(),
            ..TxResponse::default()
        };

        let Some(TransitionStatus::Committed(committed)) = committed_status(tx) else {
            panic!("expected a committed status");
        };
        assert_eq!(committed.height, 1234);
        assert_eq!(committed.error, None);
    }

    #[test]
    fn should_carry_the_code_of_a_committed_transaction_that_failed() {
        let tx = TxResponse {
            height: 77,
            tx: "AAEC".to_string(),
            tx_result: ExecTxResult {
                code: 40100,
                data: "nonce already present".to_string(),
                ..ExecTxResult::default()
            },
            ..TxResponse::default()
        };

        let Some(TransitionStatus::Committed(committed)) = committed_status(tx) else {
            panic!("expected a committed status");
        };
        assert_eq!(committed.height, 77);
        let error = committed
            .error
            .expect("a failed transition carries its error");
        assert_eq!(error.code, 40100);
    }

    #[test]
    fn should_read_an_empty_transaction_index_answer_as_not_committed() {
        assert_eq!(committed_status(TxResponse::default()), None);
    }

    #[test]
    fn should_read_only_missing_hashes_as_not_found() {
        let not_found = DapiError::TenderdashClientError(TenderdashStatus::new(
            -32603,
            Some("tx (ABCD) not found".to_string()),
            None,
        ));
        assert!(is_not_found(&not_found));
        assert!(is_not_found(&DapiError::NotFound("missing".to_string())));

        let unavailable = DapiError::TenderdashClientError(TenderdashStatus::new(
            -32603,
            Some("connection refused".to_string()),
            None,
        ));
        assert!(!is_not_found(&unavailable));
        assert!(!is_not_found(&DapiError::Unavailable("down".to_string())));
    }
}
