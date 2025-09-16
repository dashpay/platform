use clap::Subcommand;

use crate::error::CliResult;

pub mod protocol;
pub mod state_transition;

#[derive(Subcommand, Debug)]
pub enum PlatformCommand {
    /// Platform state transition helpers
    #[command(subcommand)]
    StateTransition(state_transition::StateTransitionCommand),
    /// Fetch protocol version upgrade state summary
    ProtocolUpgradeState(protocol::UpgradeStateCommand),
    /// Fetch protocol version upgrade vote status details
    ProtocolUpgradeVoteStatus(protocol::UpgradeVoteStatusCommand),
}

pub async fn run(url: &str, command: PlatformCommand) -> CliResult<()> {
    match command {
        PlatformCommand::StateTransition(command) => state_transition::run(url, command).await,
        PlatformCommand::ProtocolUpgradeState(command) => {
            protocol::run_upgrade_state(url, command).await
        }
        PlatformCommand::ProtocolUpgradeVoteStatus(command) => {
            protocol::run_upgrade_vote_status(url, command).await
        }
    }
}
