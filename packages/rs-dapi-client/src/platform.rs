//! Platform DAPI requests.

use dapi_grpc::platform::v0::platform_client::PlatformClient;
use tonic::transport::Channel;

mod get_identity;

type PlatformGrpcClient = PlatformClient<Channel>;
