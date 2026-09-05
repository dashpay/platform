use crate::abci::app::PlatformApplication;
use crate::abci::handler;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::SnapshotManager;
use crate::rpc::core::CoreRPCLike;
use crate::utils::spawn_blocking_task_with_name_if_supported;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::abci_application_server as grpc_abci_server;
use tenderdash_abci::proto::tonic;

/// AbciApp is an implementation of gRPC ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Platform
    platform: Arc<Platform<C>>,
    core_rpc: Arc<C>,
    /// The snapshot manager, pinning checkpoints that are being served to peers.
    ///
    /// Shared (`Arc`) rather than owned: this application never sees blocks, so it
    /// cannot expire the pins of abandoned transfers itself — `server::start` keeps a
    /// clone and sweeps expired pins on a timer.
    snapshot_manager: Arc<SnapshotManager>,
}

impl<C> PlatformApplication<C> for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn platform(&self) -> &Platform<C> {
        self.platform.as_ref()
    }
}

impl<C> CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    /// Create new ABCI app
    pub fn new(
        platform: Arc<Platform<C>>,
        core_rpc: Arc<C>,
        snapshot_manager: Arc<SnapshotManager>,
    ) -> Self {
        Self {
            platform,
            core_rpc,
            snapshot_manager,
        }
    }
}

impl<C> Debug for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<CheckTxAbciApplication>")
    }
}

#[async_trait]
impl<C> grpc_abci_server::AbciApplication for CheckTxAbciApplication<C>
where
    C: CoreRPCLike + Send + Sync + 'static,
{
    async fn echo(
        &self,
        request: tonic::Request<proto::RequestEcho>,
    ) -> Result<tonic::Response<proto::ResponseEcho>, tonic::Status> {
        let response = handler::echo(self, request.into_inner()).map_err(error_into_status)?;

        Ok(tonic::Response::new(response))
    }

    async fn check_tx(
        &self,
        request: tonic::Request<proto::RequestCheckTx>,
    ) -> Result<tonic::Response<proto::ResponseCheckTx>, tonic::Status> {
        let platform = Arc::clone(&self.platform);
        let core_rpc = Arc::clone(&self.core_rpc);

        let proto_request = request.into_inner();

        let check_tx_type = proto::CheckTxType::try_from(proto_request.r#type)
            .map_err(|_| tonic::Status::invalid_argument("invalid check tx type"))?;

        let thread_name = match check_tx_type {
            proto::CheckTxType::New => "check_tx",
            proto::CheckTxType::Recheck => "re_check_tx",
        };

        spawn_blocking_task_with_name_if_supported(thread_name, move || {
            let response = handler::check_tx(&platform, &core_rpc, proto_request)
                .map_err(error_into_status)?;

            Ok(tonic::Response::new(response))
        })?
        .await
        .map_err(|error| tonic::Status::internal(format!("check tx panics: {}", error)))?
    }

    async fn list_snapshots(
        &self,
        request: tonic::Request<proto::RequestListSnapshots>,
    ) -> Result<tonic::Response<proto::ResponseListSnapshots>, tonic::Status> {
        // Checkpoint metadata reads are synchronous rocksdb work; requests are
        // peer-controlled, so keep them off the async workers (same pattern as check_tx)
        let platform = Arc::clone(&self.platform);
        let proto_request = request.into_inner();

        spawn_blocking_task_with_name_if_supported("list_snapshots", move || {
            handler::list_snapshots(platform.as_ref(), proto_request)
                .map(tonic::Response::new)
                .map_err(error_into_status)
        })?
        .await
        .map_err(|error| tonic::Status::internal(format!("list snapshots panics: {}", error)))?
    }

    async fn load_snapshot_chunk(
        &self,
        request: tonic::Request<proto::RequestLoadSnapshotChunk>,
    ) -> Result<tonic::Response<proto::ResponseLoadSnapshotChunk>, tonic::Status> {
        // Chunk generation traverses the checkpoint's grovedb and encodes a replication
        // chunk — synchronous, potentially large, and peer-controlled. Run it on the
        // blocking pool so concurrent snapshot consumers cannot occupy the async workers
        // and delay unrelated gRPC traffic (same pattern as check_tx).
        let platform = Arc::clone(&self.platform);
        let snapshot_manager = Arc::clone(&self.snapshot_manager);
        let proto_request = request.into_inner();

        spawn_blocking_task_with_name_if_supported("load_snapshot_chunk", move || {
            handler::load_snapshot_chunk(platform.as_ref(), &snapshot_manager, proto_request)
                .map(tonic::Response::new)
                .map_err(error_into_status)
        })?
        .await
        .map_err(|error| {
            tonic::Status::internal(format!("load snapshot chunk panics: {}", error))
        })?
    }
}

pub fn error_into_status(error: Error) -> tonic::Status {
    match error {
        Error::Execution(crate::error::execution::ExecutionError::CheckTxProofVerificationBusy) => {
            tonic::Status::resource_exhausted(
                "check tx verification capacity is temporarily unavailable",
            )
        }
        error => tonic::Status::internal(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;
    use crate::rpc::core::MockCoreRPCLike;

    #[test]
    fn error_into_status_produces_internal_status() {
        let error = Error::Execution(ExecutionError::CorruptedCodeExecution("test error message"));
        let status = error_into_status(error);

        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("test error message"));
    }

    #[test]
    fn error_into_status_preserves_error_message() {
        let error = Error::Execution(ExecutionError::NotInTransaction("no active transaction"));
        let status = error_into_status(error);

        assert!(status.message().contains("no active transaction"));
    }

    #[test]
    fn error_into_status_marks_proof_capacity_as_retryable() {
        let status = error_into_status(Error::Execution(
            ExecutionError::CheckTxProofVerificationBusy,
        ));

        assert_eq!(status.code(), tonic::Code::ResourceExhausted);
        assert!(!status.message().contains("proof"));
    }

    #[test]
    fn check_tx_abci_application_debug_format() {
        // Verify the Debug implementation for CheckTxAbciApplication produces expected output
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let core_rpc = MockCoreRPCLike::new();

        let app = CheckTxAbciApplication::new(
            Arc::new(platform.platform),
            Arc::new(core_rpc),
            Arc::new(SnapshotManager::new()),
        );

        let debug_str = format!("{:?}", app);
        assert_eq!(debug_str, "<CheckTxAbciApplication>");
    }

    #[test]
    fn check_tx_abci_application_platform_returns_platform() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let core_rpc = MockCoreRPCLike::new();

        let app = CheckTxAbciApplication::new(
            Arc::new(platform.platform),
            Arc::new(core_rpc),
            Arc::new(SnapshotManager::new()),
        );

        // Just verify we can call platform() without panicking
        let _platform_ref = app.platform();
    }
}
