use crate::drive::balances::{
    total_credits_on_platform_path_query, TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
};
use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT;
use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::{Drive, RootTree};
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::Query;
use crate::verify::RootHash;
use dpp::block::epoch::{Epoch, EpochIndex};
use dpp::core_subsidy::epoch_core_reward_credits_for_distribution::epoch_core_reward_credits_for_distribution;
use dpp::fee::Credits;
use dpp::prelude::CoreBlockHeight;
use grovedb::{Element, GroveDb, PathQuery, SizedQuery};
use integer_encoding::VarInt;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies a proof for the total credits in the system and returns
    /// them if they are in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `path`: The path where elements should be.
    /// - `keys`: The requested keys.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Credits`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(crate) fn verify_total_credits_in_system_v0(
        proof: &[u8],
        core_subsidy_halving_interval: u32,
        request_activation_core_height: impl Fn() -> Result<CoreBlockHeight, Error>,
        current_core_height: CoreBlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Credits), Error> {
        let total_credits_on_platform_path_query = total_credits_on_platform_path_query();

        let (root_hash, mut proved_path_key_values) = GroveDb::verify_subset_query(
            proof,
            &total_credits_on_platform_path_query,
            &platform_version.drive.grove_version,
        )?;
        if proved_path_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::TooManyElements("We should only get back at most 1 element in the proof for the total credits in the system")));
        }

        let Some(proved_path_key_value) = proved_path_key_values.pop() else {
            return Err(Error::Proof(ProofError::IncorrectProof(
                "This proof would show that Platform has not yet been initialized".to_string(),
            )));
        };

        if proved_path_key_value.0 != total_credits_on_platform_path_query.path {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "The result of this proof is not what we asked for (path)".to_string(),
            )));
        }

        if proved_path_key_value.1 != TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "The result of this proof is not what we asked for (key)".to_string(),
            )));
        }

        let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "We are expecting an item for the total credits in platform field".to_string(),
            )));
        };

        let credits = Credits::decode_var(bytes.as_slice()).ok_or(Error::Proof(ProofError::CorruptedProof("The result of this proof does not contain an encoded var integer for total credits".to_string())))?.0;

        // we also need the path_query for the start_core_height of this unpaid epoch
        let unpaid_epoch_index = PathQuery {
            path: vec![vec![RootTree::Pools as u8]],
            query: SizedQuery {
                query: Query::new_single_key(KEY_UNPAID_EPOCH_INDEX.to_vec()),
                limit: Some(1),
                offset: None,
            },
        };

        let (_, mut proved_path_key_values) = GroveDb::verify_subset_query(
            proof,
            &unpaid_epoch_index,
            &platform_version.drive.grove_version,
        )?;

        let Some(proved_path_key_value) = proved_path_key_values.pop() else {
            return Err(Error::Proof(ProofError::IncorrectProof("This proof would show that Platform has not yet been initialized as we can not find a start index".to_string())));
        };

        if proved_path_key_value.0 != unpaid_epoch_index.path {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "The result of this proof is not what we asked for (unpaid epoch path)".to_string(),
            )));
        }

        if proved_path_key_value.1 != KEY_UNPAID_EPOCH_INDEX.to_vec() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "The result of this proof is not what we asked for (unpaid epoch key)".to_string(),
            )));
        }

        let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "We are expecting an item for the epoch index".to_string(),
            )));
        };

        let epoch_index = EpochIndex::from_be_bytes(bytes.as_slice().try_into().map_err(|_| {
            Error::Proof(ProofError::CorruptedProof(
                "epoch index invalid length".to_string(),
            ))
        })?);

        let start_core_height = if epoch_index == 0 {
            request_activation_core_height()?
        } else {
            let epoch = Epoch::new(epoch_index).map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "Epoch index out of bounds".to_string(),
                ))
            })?;

            let start_core_height_query = PathQuery {
                path: epoch.get_path_vec(),
                query: SizedQuery {
                    query: Query::new_single_key(KEY_START_BLOCK_CORE_HEIGHT.to_vec()),
                    limit: None,
                    offset: None,
                },
            };

            let (_, mut proved_path_key_values) = GroveDb::verify_subset_query(
                proof,
                &start_core_height_query,
                &platform_version.drive.grove_version,
            )?;

            let Some(proved_path_key_value) = proved_path_key_values.pop() else {
                return Err(Error::Proof(ProofError::IncorrectProof(
                    "We can not find the start core height of the unpaid epoch".to_string(),
                )));
            };

            if proved_path_key_value.0 != start_core_height_query.path {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "The result of this proof is not what we asked for (start core height path)"
                        .to_string(),
                )));
            }

            if proved_path_key_value.1 != KEY_START_BLOCK_CORE_HEIGHT.to_vec() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "The result of this proof is not what we asked for (start core height key)"
                        .to_string(),
                )));
            }

            let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "We are expecting an item for the start core height of the unpaid epoch"
                        .to_string(),
                )));
            };

            u32::from_be_bytes(bytes.as_slice().try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "start core height invalid length".to_string(),
                ))
            })?) + 1 // We need a plus one here, because we already distribute the first block on epoch change
        };

        let reward_credits_accumulated_during_current_epoch =
            epoch_core_reward_credits_for_distribution(
                start_core_height,
                current_core_height,
                core_subsidy_halving_interval,
                platform_version,
            )?;

        let total_credits = credits.checked_add(reward_credits_accumulated_during_current_epoch).ok_or(Error::Proof(ProofError::CorruptedProof("overflow while adding platform credits with reward credits accumulated during current epoch".to_string())))?;

        Ok((root_hash, total_credits))
    }
}
