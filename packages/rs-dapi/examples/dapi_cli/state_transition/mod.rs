mod monitor;
mod workflow;

use clap::Subcommand;

use crate::error::CliResult;

#[derive(Subcommand, Debug)]
pub enum StateTransitionCommand {
    /// Wait for a state transition result by hash
    Monitor(monitor::MonitorCommand),
    /// Broadcast a state transition and wait for the result
    Workflow(workflow::WorkflowCommand),
}

pub async fn run(url: &str, command: StateTransitionCommand) -> CliResult<()> {
    match command {
        StateTransitionCommand::Monitor(cmd) => monitor::run(url, cmd).await,
        StateTransitionCommand::Workflow(cmd) => workflow::run(url, cmd).await,
    }
}
