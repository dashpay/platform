//! Listing of gRPC requests used in DAPI.

use std::thread::sleep;
use std::time::Duration;

use super::TransportError;
use crate::{request_settings::AppliedRequestSettings, Uri};
use dapi_grpc::core::v0::core_client::CoreClient;

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::tonic::{self as tonic};
use futures::FutureExt;
// use tonic_web_wasm_client::Client as WasmClient;

#[derive(Clone, Debug)]
// TODO impleent WasmClient using `tonic_web_wasm_client::Client`
pub struct WasmClient;

impl tonic::client::GrpcService<tonic::body::BoxBody> for WasmClient {
    type ResponseBody = tonic::body::BoxBody;
    fn call(&mut self, request: http::Request<tonic::body::BoxBody>) -> Self::Future {
        unimplemented!()
    }
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        unimplemented!()
    }
    type Error = tonic::Status;
    type Future = std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<http::Response<tonic::body::BoxBody>, Self::Error>,
                > + Send,
        >,
    >;
}

/// Platform Client using gRPC transport.
pub type PlatformGrpcClient = PlatformClient<WasmClient>;
/// Core Client using gRPC transport.
pub type CoreGrpcClient = CoreClient<WasmClient>;

type S = futures::future::BoxFuture<'static, ()>;
/// backon::Sleeper
#[derive(Default, Clone, Debug)]
pub struct BackonSleeper {}
// TODO move somewhere else
impl backon::Sleeper for BackonSleeper {
    type Sleep = S;
    fn sleep(&self, dur: Duration) -> Self::Sleep {
        // TODO: blocking sleep is not the best solution
        sleep(dur);
        async {}.boxed()
    }
}

pub(super) fn create_channel(
    uri: Uri,
    settings: Option<&AppliedRequestSettings>,
) -> Result<WasmClient, TransportError> {
    // let host = uri.host().expect("Failed to get host from URI").to_string();
    // let client = tonic_web_wasm_client::Client::new(uri.to_string());

    Ok(WasmClient {})
}
