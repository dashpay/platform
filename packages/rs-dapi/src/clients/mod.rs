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
