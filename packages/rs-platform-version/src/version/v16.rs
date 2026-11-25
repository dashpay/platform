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
/// 2. **Properties drawn at random, inactive**: `apply_generated_from` 1
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
///    values. The function stays inactive: `CONTRACT_VERSIONS_V7`'s `random_generated_from`
///    is `None`, as in every earlier table, so registration refuses it with
///    `InvalidContractStructure` until block execution draws the values from the beacon
///    Tenderdash delivers, which a later change wires and activates. Protocol v15 refuses it
///    with the code it gives an unknown function, as it did before the function existed. The
///    call sites this extended in place (the document create and replace transformers, the
///    parser's read checks, the reference validation's referenced side, the create's
///    prepay) are inert at every version, where no type declares such a function.
///
/// 3. **Token amounts drawn at random, inactive**: format version 1 of the token transfer and
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
///    formats stay inactive: `random_token_amount` is `None` in every serialization table,
///    including `STATE_TRANSITION_SERIALIZATION_VERSIONS_V4`, which this version takes from
///    protocol version 15, so a batch carrying one is refused at decode
///    (`StateTransition::active_version_range`) until block execution settles them from the
///    beacon Tenderdash delivers. Activating them sets it in a serialization table this
///    version alone selects: a test holds every table of protocol version 15 and earlier to
///    `None`, and every table admitting them to a Drive version with the escrow and the
///    settlement.
pub const PLATFORM_V16: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_16,
    drive: DRIVE_VERSION_V11, // changed: create_initial_state_structure 5 creates the token order escrow trees; the token methods of the marketplace and the deeper escrow layer estimates (note 1); draw_random_document_properties Some(0) draws the sys.random properties of a block's documents and prices what their create prepays, which nothing calls while the function is inactive (note 2); settle_random_token_amounts Some(0) in DRIVE_TOKEN_METHOD_VERSIONS_V3 settles random token amounts, which nothing decodes while the format is inactive (note 3)
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V5, // changed: the token market contract v1 (note 1)
    ..PLATFORM_V15
};
