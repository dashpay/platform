pub mod core_client;
pub mod drive_client;
pub mod tenderdash_client;
pub mod tenderdash_websocket;

use std::time::Duration;

pub use core_client::CoreClient;
pub use drive_client::DriveClient;
pub use tenderdash_client::TenderdashClient;
pub use tenderdash_websocket::{TenderdashWebSocketClient, TransactionEvent, TransactionResult};

/// Default timeout for all Tenderdash HTTP requests
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Connection timeout for establishing HTTP connections; as we do local, 1s is enough
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// How long a connection carrying a block query stays silent before it is probed, and the
/// time between probes. Such a call has no deadline, so the probes are what ends it when
/// Tenderdash is gone.
const BLOCK_QUERY_KEEPALIVE: Duration = Duration::from_secs(15);
/// Unanswered probes after which a block query's connection is closed
const BLOCK_QUERY_KEEPALIVE_RETRIES: u32 = 4;
