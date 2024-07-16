use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::{block_execution_outcome, block_proposal};
use crate::rpc::core::CoreRPCLike;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Runs a block proposal, either from process proposal or prepare proposal.
    ///
    /// This function takes a `BlockProposal` and a `Transaction` as input and processes the block
    /// proposal. It first validates the block proposal and then processes raw state transitions,
    /// withdrawal transactions, and block fees. It also updates the validator set.
    ///
    /// # Arguments
    ///
    /// * `block_proposal` - The block proposal to be processed.
    /// * `known_from_us` - Do we know that we made this block proposal?
    /// * `transaction` - The transaction associated with the block proposal.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<BlockExecutionOutcome, Error>, Error>` - If the block proposal is
    ///   successfully processed, it returns a `ValidationResult` containing the `BlockExecutionOutcome`.
    ///   If the block proposal processing fails, it returns an `Error`. Consensus errors are returned
    ///   in the `ValidationResult`, while critical system errors are returned in the `Result`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with processing the block
    /// proposal, updating the core info, processing raw state transitions, or processing block fees.
    ///
    pub fn run_block_proposal(
        &self,
        block_proposal: block_proposal::v0::BlockProposal,
        known_from_us: bool,
        platform_state: &PlatformState,
        transaction: &Transaction,
    ) -> Result<ValidationResult<block_execution_outcome::v0::BlockExecutionOutcome, Error>, Error>
    {
        // Epoch information is always calculated with the last committed platform version
        // even if we are switching to a new version in this block.
        let last_committed_platform_version = platform_state.current_platform_version()?;

        // !!!! This EpochInfo is based on the last committed platform version
        // !!!! and will be used for the first block of the epoch.
        let epoch_info = self.gather_epoch_info(
            &block_proposal,
            transaction,
            platform_state,
            last_committed_platform_version,
        )?;

        // Create a bock state from previous committed state
        let mut block_platform_state = platform_state.clone();

        // Determine a platform version for this block
        let block_platform_version = if epoch_info.is_epoch_change_but_not_genesis()
            && platform_state.next_epoch_protocol_version()
                != platform_state.current_protocol_version_in_consensus()
        {
            // Switch to next proposed platform version if we are on the first block of the new epoch
            // and the next protocol version (locked in the previous epoch) is different from the
            // current protocol version.
            // This version will be set to the block state, and we decide on next version for next epoch
            // during block processing
            let next_protocol_version = platform_state.next_epoch_protocol_version();

            // We should panic if this node is not supported a new protocol version
            let Ok(next_platform_version) = PlatformVersion::get(next_protocol_version) else {
                panic!(
                    r#"Failed to upgrade the network protocol version {next_protocol_version}.

Please update your software to the latest version: https://docs.dash.org/platform-protocol-upgrade

Your software version: {}, latest supported protocol version: {}."#,
                    env!("CARGO_PKG_VERSION"),
                    PlatformVersion::latest().protocol_version
                );
            };

            // Set current protocol version to the block platform state
            block_platform_state.set_current_protocol_version_in_consensus(next_protocol_version);

            next_platform_version
        } else {
            // Stay on the last committed platform version
            last_committed_platform_version
        };

        // Patch platform version and run migrations if we have patches and/or
        // migrations defined for this height.
        // It modifies the protocol version to function version mapping to apply hotfixes
        // Also it performs migrations to fix corrupted state or prepare it for new features
        let block_platform_version = if let Some(patched_platform_version) = self
            .apply_platform_version_patch_and_migrate_state_for_height(
                block_proposal.height,
                &mut block_platform_state,
                transaction,
            )? {
            patched_platform_version
        } else {
            block_platform_version
        };

        match block_platform_version
            .drive_abci
            .methods
            .engine
            .run_block_proposal
        {
            0 => self.run_block_proposal_v0(
                block_proposal,
                known_from_us,
                epoch_info,
                transaction,
                platform_state,
                block_platform_state,
                block_platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "run_block_proposal".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
