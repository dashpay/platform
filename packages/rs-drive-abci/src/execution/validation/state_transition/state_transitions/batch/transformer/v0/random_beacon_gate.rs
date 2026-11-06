//! The block that switches to protocol version 15 receives no random beacon. Tenderdash applies
//! the consensus parameters a block returns from the next height on, so that block still runs
//! under Tenderdash consensus version 1, which neither recovers a beacon nor calls
//! `ApplyBlockRandomness`, and its commit carries none. Nothing would then draw what the block's
//! transitions ask for: a document created with a `sys.random` property would keep its
//! placeholder for good, and a random token amount would leave its maximum in escrow. So the
//! batch transformer refuses such a transition in that block, unpaid and before anything is read
//! or written (`RandomBeaconUnavailableError`), and the next block, which runs under consensus
//! version 2, accepts it.
//!
//! Every protocol version selects the transformer this gate sits in, and it is inert before 15:
//! [`random_beacon_withheld`] is false wherever the block's protocol version draws nothing
//! (`apply_random_beacon` is `None`), before it reads anything else.

use crate::error::Error;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use dpp::consensus::basic::state_transition::RandomBeaconUnavailableError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::{DocumentTypeRef, SystemFunction};
use dpp::data_contract::DataContract;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Whether the block `platform_state` belongs to draws random values but receives no random
/// beacon to draw them from. Its protocol version, `platform_version`, draws them
/// (`apply_random_beacon` is `Some`), while Tenderdash runs it under the consensus version of
/// the version it switches from
/// ([`PlatformStateV0Methods::tenderdash_consensus_platform_version`]), whose
/// `apply_random_beacon` is `None`: the block that switches to protocol version 15. In
/// check_tx, `platform_state` is the committed state, which runs under its own version, so this
/// is false there.
pub(super) fn random_beacon_withheld(
    platform_state: &PlatformState,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    if platform_version
        .drive_abci
        .methods
        .block_end
        .apply_random_beacon
        .is_none()
    {
        return Ok(false);
    }
    Ok(platform_state
        .tenderdash_consensus_platform_version()?
        .drive_abci
        .methods
        .block_end
        .apply_random_beacon
        .is_none())
}

/// The refusal of the first of `token_transitions` that asks for a random amount: a token
/// transfer or mint naming a random minimum amount. `None` when none does.
pub(super) fn refuse_random_token_amount<'a>(
    token_transitions: impl IntoIterator<Item = &'a TokenTransition>,
    platform_version: &PlatformVersion,
) -> Option<ConsensusError> {
    token_transitions.into_iter().find_map(|token_transition| {
        let kind = match token_transition {
            TokenTransition::Transfer(transfer) if transfer.random_minimum_amount().is_some() => {
                "transfer"
            }
            TokenTransition::Mint(mint) if mint.random_minimum_amount().is_some() => "mint",
            _ => return None,
        };
        Some(
            RandomBeaconUnavailableError::new(
                format!("The token {kind} of token {}", token_transition.token_id()),
                platform_version.protocol_version,
            )
            .into(),
        )
    })
}

/// The refusal of the first type of `contract` among `document_transitions` (the transitions of
/// a batch on `contract`, by type name) that creates a document and declares a property drawn at
/// random. `None` when none does.
pub(super) fn refuse_random_document(
    contract: &DataContract,
    document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
    platform_version: &PlatformVersion,
) -> Option<ConsensusError> {
    document_transitions
        .iter()
        .find(|(document_type_name, transitions)| {
            transitions
                .iter()
                .any(|transition| matches!(transition, DocumentTransition::Create(_)))
                && contract
                    .document_type_optional_for_name(document_type_name)
                    .is_some_and(declares_a_property_drawn_at_random)
        })
        .map(|(document_type_name, _)| {
            RandomBeaconUnavailableError::new(
                format!(
                    "The create of a document of type \"{document_type_name}\" of data contract {}",
                    contract.id()
                ),
                platform_version.protocol_version,
            )
            .into()
        })
}

/// Whether `document_type` declares a property drawn at random (`generatedFrom` with a
/// `sys.random` function), which no type parsed before protocol version 15 does.
fn declares_a_property_drawn_at_random(document_type: DocumentTypeRef) -> bool {
    document_type
        .generated_from_fields()
        .iter()
        .any(|(_, generated_from)| matches!(generated_from.function, SystemFunction::Random(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use dpp::dashcore::Network;

    /// The state of a block at `protocol_version`, switching from `switched_from` when set.
    fn block_state(
        protocol_version: u32,
        switched_from: Option<u32>,
    ) -> (PlatformState, &'static PlatformVersion) {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected a known protocol version");
        let mut state = PlatformState::default_with_protocol_versions(
            protocol_version,
            protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("expected a platform state");
        state.set_switched_from_protocol_version(switched_from);
        (state, platform_version)
    }

    fn withheld(protocol_version: u32, switched_from: Option<u32>) -> bool {
        let (state, platform_version) = block_state(protocol_version, switched_from);
        random_beacon_withheld(&state, platform_version).expect("expected the gate to decide")
    }

    #[test]
    fn should_withhold_the_beacon_only_from_the_block_switching_to_protocol_version_15() {
        let latest = PlatformVersion::latest().protocol_version;
        // The block that switches from 14 runs under consensus version 1
        assert!(withheld(latest, Some(14)));
        // Any later block, and check_tx on a committed state, run under consensus version 2
        assert!(!withheld(latest, None));
        // Inert at the last shipped protocol version, whatever it switched from: its blocks draw
        // nothing
        assert!(!withheld(14, Some(13)));
        assert!(!withheld(14, None));
    }
}
