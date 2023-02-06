use drive::drive::config::DriveConfig;

/// Configuration for Dash Core RPC client
pub struct CoreRpcConfig {
    /// Core RPC client url
    pub url: String,

    /// Core RPC client username
    pub username: String,

    /// Core RPC client password
    pub password: String,
}

/// Configuration for Dash Core related things
pub struct CoreConfig {
    /// Core RPC config
    pub rpc: CoreRpcConfig,
}

/// Platform configuration
#[derive(Clone, Debug)]
pub struct PlatformConfig {
    /// Drive configuration
    pub drive: Option<DriveConfig>,

    /// Dash Core config
    pub core: CoreConfig,

    /// Should we verify sum trees? Useful to set as no for tests
    pub verify_sum_trees: bool,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            drive: Default::default(),
            core: CoreConfig {
                rpc: CoreRpcConfig {
                    url: "127.0.0.1".to_owned(),
                    username: "".to_owned(),
                    password: "".to_owned(),
                },
            },
            verify_sum_trees: true,
        }
    }
}
