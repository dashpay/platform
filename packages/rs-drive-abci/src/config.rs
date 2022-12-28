use drive::drive::config::DriveConfig;

/// Configuration for Dash Core related things
pub struct CoreConfig {
    /// Core RPC client url
    pub rpc_url: String,

    /// Core RPC client username
    pub rpc_username: String,

    /// Core RPC client password
    pub rpc_password: String,
}

/// Platform configuration
pub struct PlatformConfig {
    /// Drive configuration
    pub drive: Option<DriveConfig>,

    /// Dash Core config
    pub core: CoreConfig,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            drive: None,
            core: CoreConfig {
                rpc_url: "127.0.0.1".to_owned(),
                rpc_username: "".to_owned(),
                rpc_password: "".to_owned(),
            },
        }
    }
}
