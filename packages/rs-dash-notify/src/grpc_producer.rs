use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::PlatformEventsCommand;
use dapi_grpc::tonic::Status;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::event_mux::unbounded_sender_sink;
use crate::event_mux::EventMux;

/// A reusable gRPC producer that bridges a Platform gRPC client with an [`EventMux`].
///
/// Creates bi-directional channels, subscribes upstream using the provided client,
/// and forwards commands/responses between the upstream stream and the mux.
pub struct GrpcPlatformEventsProducer;

impl GrpcPlatformEventsProducer {
    /// Connect the provided `client` to the `mux` and forward messages until completion.
    ///
    /// The `ready` receiver is used to signal when the producer has started.
    pub async fn run<C>(
        mux: EventMux,
        mut client: PlatformClient<C>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), Status>
    where
        // C: DapiRequestExecutor,
        C: dapi_grpc::tonic::client::GrpcService<dapi_grpc::tonic::body::Body>,
        C::Error: Into<dapi_grpc::tonic::codegen::StdError>,
        C::ResponseBody: dapi_grpc::tonic::codegen::Body<Data = dapi_grpc::tonic::codegen::Bytes>
            + Send
            + 'static,
        <C::ResponseBody as dapi_grpc::tonic::codegen::Body>::Error:
            Into<dapi_grpc::tonic::codegen::StdError> + Send,
    {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<PlatformEventsCommand>();
        tracing::debug!("connecting gRPC producer to upstream");
        let resp_stream = client
            .subscribe_platform_events(UnboundedReceiverStream::new(cmd_rx))
            .await?;
        let cmd_sink = unbounded_sender_sink(cmd_tx);
        let resp_rx = resp_stream.into_inner();

        tracing::debug!("registering gRPC producer with mux");
        let producer = mux.add_producer().await;
        tracing::debug!("gRPC producer connected to mux and ready, starting forward loop");
        ready.send(()).ok();
        producer.forward(cmd_sink, resp_rx).await;
        Ok(())
    }
}
