use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_contract_versions::v8::CONTRACT_VERSIONS_V8;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v5::STATE_TRANSITION_SERIALIZATION_VERSIONS_V5;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v12::DRIVE_ABCI_METHOD_VERSIONS_V12;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v11::DRIVE_VERSION_V11;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v5::SYSTEM_DATA_CONTRACT_VERSIONS_V5;
use crate::version::v15::PLATFORM_V15;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_16: ProtocolVersion = 16;

/// Introduced as the activation gate for the consensus changes of the 5.2 line.
///
/// Every table this version does not name is protocol version 15's (`..PLATFORM_V15`), so a
/// change made in place to an unreleased protocol version 15 table reaches this version too.
/// Each change that needs this gate lands in its own follow-up, adds a numbered note here and
/// names the table it bumps in `PLATFORM_V16` with a `// changed:` comment.
///
/// v16 hosts these consensus changes:
///
/// 1. **Token marketplace state**: the token marketplace starts with this version
///    (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`). This version admits tradeable tokens
///    (`marketplaceRules.tradeMode` `Tradeable`) and creates the escrow trees and the token
///    market contract that listings and offers will use; the transitions that place and fill
///    orders come in follow-ups. Protocol v15 and earlier hold none of it (v15 note 17).
///    - A token configuration, or a configuration update, that makes a token tradeable is
///      admitted; before this version it is refused unpaid as not active.
///    - `create_initial_state_structure` 5 (`DRIVE_VERSION_V11`) creates the two escrow trees at
///      genesis, and the upgrade to this version (`transition_to_version_16`) inserts them
///      through the same helper, `Drive::insert_token_order_escrow_trees`.
///    - `calculate_total_tokens_balance` 2 counts the escrowed tokens in the token
///      conservation check, and the escrow methods (`order_escrow`) are active
///      (`DRIVE_TOKEN_METHOD_VERSIONS_V3`).
///    - The escrow keys sit deeper in their layers than the keys beside them, so the cost
///      estimators of the balances, the token balances, the prefunded specialized balances and
///      the contract fee pots keep a deeper description of those layers (`DRIVE_VERSION_V11`).
///    - The token market contract v1 (`token_market: 1`, `SYSTEM_DATA_CONTRACT_VERSIONS_V5`) is
///      registered at genesis and stored without storage flags by the upgrade. Its `ask` and
///      `bid` types hold the open orders.
///
/// 2. **Properties drawn at random**: `apply_generated_from` 1
///    (`CONTRACT_VERSIONS_V7`, as at protocol version 15) parses the `generatedFrom` function
///    `sys.random.uniform`, which takes no `params` (they may be left out, or be an empty
///    list), on an integer property of at most 64 bits declaring both a `minimum` and a
///    `maximum` inside its type and no `enum`, `const`, `exclusiveMinimum`, `exclusiveMaximum`
///    or `multipleOf`: the platform draws its value from that range, both included, out of
///    the block's random beacon once the block's transitions are accepted
///    (`draw_random_document_properties` 0, `DRIVE_VERSION_V11`, `None` in every earlier Drive
///    version). A create writes the placeholder, the range's minimum, and refuses a sent
///    value; a replace keeps the stored value, written back when left out and refused when
///    sent with another (`DocumentPropertyNotGeneratedError`); clients leave the property out
///    and take it from the proof. Registration refuses it on an indexOnly type, on a type that
///    keeps history or has a contested index, in a unique index, summed or averaged, read by a
///    reference or a rule, by an `encryptedFor` key, or by another type through a reference (a
///    derived index property, a lookup, a `where` entry). A create of a type indexing such a
///    property prepays, at Drive's estimate without reading state, the entries those indexes
///    take, which the draw writes again unbilled (`prepay_random_document_index_entries` 0).
///    The compiled form of a contract (format version 2) stores the range with the
///    declaration, and reading one back refuses a range upside down or of more than 2^64
///    values. Registration admits the function: `CONTRACT_VERSIONS_V8`'s
///    `random_generated_from` is `Some(0)` (`None` in every earlier table, where registration
///    refuses it with `InvalidContractStructure`), since block execution draws the values from
///    the beacon Tenderdash delivers (note 4). Protocol v15 refuses it with the code it gives
///    an unknown function, as it did before the function existed. The call sites this
///    extended in place (the document create and replace transformers, the parser's read
///    checks, the reference validation's referenced side, the create's prepay) are inert at
///    every earlier version, where no type declares such a function.
///
/// 3. **Token amounts drawn at random**: format version 1 of the token transfer and
///    the token mint (`TokenTransferTransitionV1`, `TokenMintTransitionV1`) carries
///    `random_minimum_amount`: the amount that reaches the recipient is drawn from it to
///    `amount`, both included, out of the block's random beacon once the block's transitions
///    are accepted. Structure validation holds `1 <= minimum < amount`
///    (`InvalidTokenAmountError`) and refuses a random mint that is a group action or names
///    no recipient (`RandomTokenMintNotAllowedError`). Executing one reserves `amount` in the
///    token marketplace escrow of the token (note 1; a transfer takes it off the sender, a
///    mint adds it to the supply), writes the recipient's balance as it stands, and writes
///    the history document holding the minimum under an id that also hashes the minimum and
///    the maximum (`token_history_document_id`), so a proof binds the range; the transition
///    prepays, at Drive's estimate without reading state, the history index entries the
///    settlement rewrites. The settlement (`settle_random_token_amounts` 0, in
///    `DRIVE_TOKEN_METHOD_VERSIONS_V3` with the escrow) removes the reservation, pays out the
///    drawn amount, releases the rest and rewrites the history's amount, unbilled. The
///    formats are active from this version, whose block execution settles them from the
///    beacon Tenderdash delivers (note 4): `STATE_TRANSITION_SERIALIZATION_VERSIONS_V5`,
///    selected by this version alone, sets `random_token_amount` to `Some(0)`, and every
///    earlier table keeps it `None`, so a batch carrying one is refused at decode there
///    (`StateTransition::active_version_range`). A test holds every table admitting them to a
///    Drive version with the escrow and the settlement.
///
/// 4. **Same-block randomness**: `tenderdash_consensus_version` is 3, which consensus
///    parameter update 2 sends Tenderdash on the block that switches to this version, so
///    Tenderdash recovers a random beacon from each block's prevotes from the next height on,
///    hands it to Drive in `ApplyBlockRandomness` before the precommit (or before
///    `FinalizeBlock` with the commit's beacon), and has the precommits sign the app hash Drive
///    returns. Consensus version 2 (protocol version 15, proposer liveness) stays as it is:
///    version 3 adds the beacon on top of it. `DRIVE_ABCI_METHOD_VERSIONS_V12` sets
///    `block_end.apply_random_beacon` to `Some(0)`: a block collects the documents it creates
///    and its random token transfers and mints, and the beacon fills their `sys.random`
///    properties (`draw_random_document_properties`) and settles their amounts
///    (`settle_random_token_amounts`). Every earlier table keeps it `None` and a consensus
///    version below 3, where nothing is collected and Tenderdash sends no beacon.
///
///    The block that switches to this version still runs under consensus version 2, as
///    Tenderdash applies the consensus parameters a block returns from the next height on, so
///    it receives no beacon and nothing would draw what it asks for. The batch transformer
///    refuses, unpaid, what would ask that block for a random value: a random token transfer
///    or mint, and a create of a document whose type declares a `sys.random` property
///    (`RandomBeaconUnavailableError`). Both are accepted from the next block on.
///    `run_block_proposal` records the protocol version a block switches from on the block's
///    platform state (`switched_from_protocol_version`, never saved, and cleared when the
///    block is committed), whose `tenderdash_consensus_platform_version` the gate reads.
///    From consensus version 3 on, `FinalizeBlock` also refuses a commit whose app hash is
///    not the state after the block's draws, or whose block id carries a state id other than
///    the hash of the header's state id with that app hash.
pub const PLATFORM_V16: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_16,
    drive: DRIVE_VERSION_V11, // changed: create_initial_state_structure 5 creates the token order escrow trees; the token methods of the marketplace and the deeper escrow layer estimates (note 1); draw_random_document_properties Some(0) draws the sys.random properties of a block's documents and prices what their create prepays, (note 2); settle_random_token_amounts Some(0) in DRIVE_TOKEN_METHOD_VERSIONS_V3 settles random token amounts (note 3)
    dpp: DPPVersion {
        contract_versions: CONTRACT_VERSIONS_V8, // changed: random_generated_from 0 admits sys.random.uniform (note 2)
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V5, // changed: random_token_amount 0 activates version 1 token transfers and mints (note 3)
        ..PLATFORM_V15.dpp
    },
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V12, // changed: block_end.apply_random_beacon 0 applies each block's random beacon (note 4)
        ..PLATFORM_V15.drive_abci
    },
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 3, // changed: Tenderdash recovers each block's random beacon and calls ApplyBlockRandomness (note 4)
        ..PLATFORM_V15.consensus
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V5, // changed: the token market contract v1 (note 1)
    ..PLATFORM_V15
};
