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
use std::future::Future;
use std::time::Duration;
use tracing::debug;

use crate::clients::tenderdash_client::TxResponse;

/// How long the answer waits for the node's status to fill its metadata. The status is
/// already known by then, so a slow status read leaves the metadata out rather than delaying
/// the answer.
const NODE_VIEW_METADATA_TIMEOUT: Duration = Duration::from_secs(2);

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
        let metadata = self
            .node_view_metadata()
            .await
            .filter(|metadata| metadata_reaches(&status, metadata));

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
        lookup_sequence(
            || self.committed_state_transition(hash_base64),
            || async move {
                match self.tenderdash_client.unconfirmed_tx(hash_base64).await {
                    Ok(_) => Ok(true),
                    Err(error) if is_missing_from_mempool(&error) => Ok(false),
                    Err(error) => Err(error),
                }
            },
        )
        .await
    }

    /// The transition as this node's transaction index holds it, or `None` when it does not.
    async fn committed_state_transition(
        &self,
        hash_base64: &str,
    ) -> Result<Option<TransitionStatus>, DapiError> {
        match self.tenderdash_client.tx(hash_base64.to_string()).await {
            Ok(tx) => Ok(committed_status(tx)),
            Err(error) if is_missing_from_index(&error) => Ok(None),
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
        let status = match tokio::time::timeout(
            NODE_VIEW_METADATA_TIMEOUT,
            self.get_status_impl(request),
        )
        .await
        {
            Ok(Ok(response)) => response.into_inner(),
            Ok(Err(error)) => {
                debug!(%error, "get_recent_state_transition_status: node status unavailable");
                return None;
            }
            Err(_) => {
                debug!(
                    "get_recent_state_transition_status: node status too slow; answering without it"
                );
                return None;
            }
        };
        let StatusResponseVersion::V0(status) = status.version?;
        // A status without the chain has no height to report: leave the metadata out rather
        // than report height 0.
        let chain = status.chain?;
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

/// The order of the lookups: the transaction index, then the mempool, then the transaction
/// index again, so a transition that leaves the mempool for a block between the first two reads
/// is found by the third instead of being reported unknown. `committed` reads the index (`None`
/// when it does not have the hash); `in_mempool` reads the mempool.
async fn lookup_sequence<C, CF, M, MF>(
    mut committed: C,
    in_mempool: M,
) -> Result<TransitionStatus, DapiError>
where
    C: FnMut() -> CF,
    CF: Future<Output = Result<Option<TransitionStatus>, DapiError>>,
    M: FnOnce() -> MF,
    MF: Future<Output = Result<bool, DapiError>>,
{
    if let Some(status) = committed().await? {
        return Ok(status);
    }
    if in_mempool().await? {
        return Ok(TransitionStatus::Pending(Pending {}));
    }
    if let Some(status) = committed().await? {
        return Ok(status);
    }
    Ok(TransitionStatus::Unknown(Unknown {}))
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

/// Whether `metadata` is from at or above the block a committed answer names.
///
/// The node's status is cached until a new block's event clears it, and that event can reach
/// DAPI after Tenderdash's transaction index already holds the block. A status read before
/// then is a block behind the answer, so it is left out rather than report the node below the
/// block the answer names.
fn metadata_reaches(status: &TransitionStatus, metadata: &ResponseMetadata) -> bool {
    match status {
        TransitionStatus::Committed(committed) => metadata.height >= committed.height,
        TransitionStatus::Pending(_) | TransitionStatus::Unknown(_) => true,
    }
}

/// JSON-RPC's "internal error" code, which Tenderdash gives every error one of its RPC
/// methods returns, a lookup that found nothing among them. A call it could not route
/// ("Method not found", -32601) or refused as malformed carries another code, and fails the
/// lookup.
const TENDERDASH_METHOD_ERROR_CODE: i64 = -32603;

/// The message of an error a Tenderdash RPC method itself returned.
fn tenderdash_method_error(error: &DapiError) -> Option<&str> {
    match error {
        DapiError::TenderdashClientError(status) if status.code == TENDERDASH_METHOD_ERROR_CODE => {
            status.message.as_deref().map(str::trim)
        }
        _ => None,
    }
}

/// Whether Tenderdash's `tx` failed because its transaction index does not have the hash.
///
/// Tenderdash answers `tx (<HASH>) not found, err: <cause>` both when the index has no entry
/// and when reading the entry failed (a database or decoding error). Only a lookup with no
/// cause is a missing hash: Go formats the absent cause as `%!w(<nil>)`. A failed read must
/// fail the call, or a committed transition whose entry could not be read would be reported
/// unknown.
fn is_missing_from_index(error: &DapiError) -> bool {
    tenderdash_method_error(error).is_some_and(|message| {
        message
            .strip_prefix("tx (")
            .and_then(|rest| rest.strip_suffix(") not found, err: %!w(<nil>)"))
            .is_some_and(|hash| !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit()))
    })
}

/// Whether Tenderdash's `unconfirmed_tx` failed because its mempool does not have the hash:
/// `transaction <HASH> not found`.
fn is_missing_from_mempool(error: &DapiError) -> bool {
    tenderdash_method_error(error).is_some_and(|message| {
        message.starts_with("transaction ") && message.ends_with(" not found")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::ExecTxResult;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    fn committed_at(height: u64) -> TransitionStatus {
        TransitionStatus::Committed(Committed {
            height,
            error: None,
        })
    }

    /// Run the lookup order against index answers given in order and one mempool answer, and
    /// report the status with how many index reads it took.
    async fn run_sequence(
        index_answers: Vec<Result<Option<TransitionStatus>, DapiError>>,
        mempool_answer: Result<bool, DapiError>,
    ) -> (Result<TransitionStatus, DapiError>, usize) {
        let answers = RefCell::new(VecDeque::from(index_answers));
        let reads = RefCell::new(0usize);
        let status = lookup_sequence(
            || {
                *reads.borrow_mut() += 1;
                let answer = answers
                    .borrow_mut()
                    .pop_front()
                    .expect("the test gives an answer for every index read");
                async move { answer }
            },
            || async move { mempool_answer },
        )
        .await;
        let reads = *reads.borrow();
        (status, reads)
    }

    #[tokio::test]
    async fn should_answer_committed_from_the_first_index_read() {
        let (status, reads) = run_sequence(vec![Ok(Some(committed_at(10)))], Ok(false)).await;
        assert_eq!(status.expect("an answer"), committed_at(10));
        assert_eq!(reads, 1);
    }

    #[tokio::test]
    async fn should_answer_pending_when_the_mempool_holds_it() {
        let (status, reads) = run_sequence(vec![Ok(None)], Ok(true)).await;
        assert_eq!(
            status.expect("an answer"),
            TransitionStatus::Pending(Pending {})
        );
        assert_eq!(reads, 1);
    }

    #[tokio::test]
    async fn should_find_a_transition_that_reached_a_block_between_the_reads() {
        let (status, reads) =
            run_sequence(vec![Ok(None), Ok(Some(committed_at(11)))], Ok(false)).await;
        assert_eq!(status.expect("an answer"), committed_at(11));
        assert_eq!(reads, 2);
    }

    #[tokio::test]
    async fn should_answer_unknown_when_neither_index_nor_mempool_has_it() {
        let (status, reads) = run_sequence(vec![Ok(None), Ok(None)], Ok(false)).await;
        assert_eq!(
            status.expect("an answer"),
            TransitionStatus::Unknown(Unknown {})
        );
        assert_eq!(reads, 2);
    }

    #[tokio::test]
    async fn should_fail_rather_than_answer_unknown_when_a_lookup_fails() {
        let (status, _) = run_sequence(
            vec![Ok(None)],
            Err(DapiError::Unavailable("tenderdash down".to_string())),
        )
        .await;
        assert!(matches!(status, Err(DapiError::Unavailable(_))));

        let (status, _) = run_sequence(
            vec![Err(DapiError::Unavailable("tenderdash down".to_string()))],
            Ok(false),
        )
        .await;
        assert!(matches!(status, Err(DapiError::Unavailable(_))));
    }

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

    fn tenderdash_error(code: i64, message: &str) -> DapiError {
        DapiError::TenderdashClientError(TenderdashStatus::new(
            code,
            Some(message.to_string()),
            None,
        ))
    }

    #[test]
    fn should_read_only_a_missing_hash_as_absent_from_the_transaction_index() {
        // Tenderdash's own text when the index has no entry: Go prints the nil cause this way.
        assert!(is_missing_from_index(&tenderdash_error(
            -32603,
            "tx (ABCD) not found, err: %!w(<nil>)"
        )));

        // The same message wrapping a failed read is not a missing hash: the entry may exist.
        assert!(!is_missing_from_index(&tenderdash_error(
            -32603,
            "tx (ABCD) not found, err: failed to read tx result: database unavailable"
        )));
        assert!(!is_missing_from_index(&tenderdash_error(
            -32603,
            "tx (ABCD) not found, err: error reading TxResult: unexpected EOF"
        )));
        // The mempool's message, a failure and an unrouted method are not a missing hash.
        assert!(!is_missing_from_index(&tenderdash_error(
            -32603,
            "transaction ABCD not found"
        )));
        assert!(!is_missing_from_index(&tenderdash_error(
            -32603,
            "transaction querying is disabled on this node due to the KV event sink being disabled"
        )));
        assert!(!is_missing_from_index(&tenderdash_error(
            -32601,
            "Method not found"
        )));
        assert!(!is_missing_from_index(&DapiError::Unavailable(
            "down".to_string()
        )));
    }

    #[test]
    fn should_read_only_a_missing_hash_as_absent_from_the_mempool() {
        assert!(is_missing_from_mempool(&tenderdash_error(
            -32603,
            "transaction ABCD not found"
        )));

        assert!(!is_missing_from_mempool(&tenderdash_error(
            -32603,
            "tx (ABCD) not found, err: <nil>"
        )));
        assert!(!is_missing_from_mempool(&tenderdash_error(
            -32603,
            "connection refused"
        )));
        // A node whose Tenderdash does not serve `unconfirmed_tx` must not answer unknown for a
        // transition that may be pending.
        assert!(!is_missing_from_mempool(&tenderdash_error(
            -32601,
            "Method not found"
        )));
        assert!(!is_missing_from_mempool(&DapiError::NotFound(
            "missing".to_string()
        )));
    }

    #[test]
    fn should_leave_out_metadata_below_the_block_a_committed_answer_names() {
        let at = |height| ResponseMetadata {
            height,
            ..ResponseMetadata::default()
        };
        assert!(!metadata_reaches(&committed_at(100), &at(99)));
        assert!(metadata_reaches(&committed_at(100), &at(100)));
        assert!(metadata_reaches(&committed_at(100), &at(101)));
        // Nothing to fall behind for an answer that names no block.
        assert!(metadata_reaches(
            &TransitionStatus::Pending(Pending {}),
            &at(0)
        ));
        assert!(metadata_reaches(
            &TransitionStatus::Unknown(Unknown {}),
            &at(0)
        ));
    }
}
