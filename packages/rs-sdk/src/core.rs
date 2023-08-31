use drive_abci::rpc::core::CoreRPCLike;

// TODO implement it here
pub type CoreClient = Box<dyn CoreRPCLike + Send + Sync>;
