use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v4::DOCUMENT_VERSIONS_V4;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v2::STATE_TRANSITION_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_versions::v4::STATE_TRANSITION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_token_versions::v3::TOKEN_VERSIONS_V3;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v2::DRIVE_ABCI_STRUCTURE_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v11::DRIVE_ABCI_VALIDATION_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::fee::v3::FEE_VERSION3;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v3::SYSTEM_DATA_CONTRACT_VERSIONS_V3;
use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// Introduced as the activation gate for the consensus changes of the 4.3
/// line. Functionally identical to v14 at introduction: the same component
/// version structs, no behavior change. Each change that needs this gate
/// lands in its own follow-up and bumps the component table it consumes here;
/// keeping v15 == v14 until then lets mixed-version validators agree.
///
/// v15 hosts these consensus changes:
///
/// 1. **Chain locks signed by quorums Platform does not hold yet**: a block
///    proposal's chain lock is checked against the chain lock quorums Platform
///    fetched at its last committed core height. When a quorum was mined after
///    that height, inside the same DKG interval, or when the signing height
///    lies between the two stored quorum sets after a jump of the core height,
///    the chain lock is signed by quorums Platform does not hold, and
///    `verify_chain_lock_locally` 0 rejected it although it was valid, failing
///    rounds until Core produced a chain lock Platform could verify or one
///    signed past the DKG interval. `verify_chain_lock_locally` 1
///    (`DRIVE_ABCI_METHOD_VERSIONS_V11`) rejects locally only when Platform
///    holds exactly the quorums active at the signing height, using Dash Core's
///    DKG mining schedule (`core_chain_lock.dkg_mining_schedules`), and leaves
///    the other chain locks to Core. `verify_chain_lock_through_core` 1 has
///    Core's `verifychainlock` check the signature of every chain lock left to
///    Core before `submitchainlock`, which skips that check for a height Core
///    already has a chain lock at or above, and reports a failure as Core's
///    verdict. So v15 accepts valid chain locks that v14 rejected, and rejects
///    forged ones that v14 left to `submitchainlock`'s early return. Protocol
///    v14 and earlier
///    keep `verify_chain_lock_locally` 0 and `verify_chain_lock_through_core` 0,
///    so the blocks committed under them replay with the same verdicts.
///
/// 2. **Proofs of possession of the keys an identity update adds**: the
///    identity's signature over an identity update does not cover the
///    signatures of the keys it adds. The identity update
///    `advanced_structure` 1 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuses,
///    without a fee and without using the identity's nonce, an update whose
///    added key's proof of possession does not verify (a key whose public key
///    data does not decode fails it too), where 0 charged it; a revision
///    mismatch is still charged. Protocol v14 and
///    earlier keep `advanced_structure` 0, so the blocks committed under them
///    replay with the same results. check_tx verifies these proofs at every
///    protocol version, which decides mempool admission only.
///
/// 3. **Yes/no vote polls**: a second kind of masternode vote poll, which asks
///    "should this happen?" and passes on a supermajority of yes over yes plus
///    no with a minimum of voting power cast. A masternode vote naming a yes/no
///    poll is active from this version (`StateTransition::active_version_range`);
///    the polls live under the votes tree's decisions branch, which genesis
///    (`create_initial_state_structure` 4 behind `>= 15`) and
///    `transition_to_version_15` create. Masternode vote `transform_into_action`
///    1 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) looks up the previous answer, and
///    `DRIVE_VERSION_V10` registers and sweeps the votes. Opening a yes/no poll
///    (`Drive::open_yes_no_vote_poll`, `open_yes_no_vote_poll` `Some(0)`) is active
///    from this version, for an end date at most `max_yes_no_vote_poll_duration_ms`
///    (four weeks, a `SystemLimits` field read only by the opener) after the
///    opening block, and its end date entry goes through
///    `add_vote_poll_end_date_query_operations` 1, which adds only the entry when
///    the batch already queues the end date's tree with other storage flags.
///    Nothing calls the opener yet, so at this version every yes/no vote is
///    refused for want of a poll.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: vote method versions v4: register_identity_vote 1 registers a yes/no vote, remove_all_votes_given_by_identities 1 also removes the yes/no votes of a masternode that left the list, open_yes_no_vote_poll Some(0) makes opening a yes/no poll active and add_vote_poll_end_date_query_operations 1 adds only the entry when the batch already queues the end date's tree with other storage flags
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: verify_chain_lock_locally 1 rejects a chain lock locally only when Platform holds the quorums active at its signing height, and verify_chain_lock_through_core 1 has Core's verifychainlock check the others before submitchainlock
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: identity update advanced_structure 1 refuses unpaid an identity update whose added key's proof of possession does not verify; masternode vote transform_into_action 1 looks up a masternode's previous answer on a yes/no poll
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3,
        query: DRIVE_ABCI_QUERY_VERSIONS_V2,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2,
        state_transitions: STATE_TRANSITION_VERSIONS_V4,
        contract_versions: CONTRACT_VERSIONS_V6,
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V3,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3,
    fee_version: FEE_VERSION3,
    system_limits: SYSTEM_LIMITS_V4,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
