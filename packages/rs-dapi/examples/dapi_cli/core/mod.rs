use clap::Subcommand;

use crate::error::CliResult;

pub mod block_hash;
pub mod chainlocks;
pub mod masternode;
pub mod transactions;

#[derive(Subcommand, Debug)]
pub enum CoreCommand {
    /// Get block hash by height
    BlockHash(block_hash::BlockHashCommand),
    /// Stream Core transactions with proofs
    Transactions(transactions::TransactionsCommand),
    /// Stream masternode list diffs
    Masternode(masternode::MasternodeCommand),
    /// Stream chain locks and corresponding block headers
    ChainLocks(chainlocks::ChainLocksCommand),
}

pub async fn run(url: &str, command: CoreCommand) -> CliResult<()> {
    match command {
        CoreCommand::BlockHash(cmd) => block_hash::run(url, cmd).await,
        CoreCommand::Transactions(cmd) => transactions::run(url, cmd).await,
        CoreCommand::Masternode(cmd) => masternode::run(url, cmd).await,
        CoreCommand::ChainLocks(cmd) => chainlocks::run(url, cmd).await,
    }
}
