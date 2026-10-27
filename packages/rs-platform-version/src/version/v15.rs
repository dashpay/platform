use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v7::CONTRACT_VERSIONS_V7;
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
use crate::version::system_data_contract_versions::v4::SYSTEM_DATA_CONTRACT_VERSIONS_V4;
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
/// 3. **DPNS v3 checks a name's create with keywords**:
///    `SYSTEM_DATA_CONTRACT_VERSIONS_V4` selects DPNS contract v3, loaded at
///    genesis (which still inserts the `dash` top-level domain). Its `domain`
///    generates `normalizedLabel` and `normalizedParentDomainName` with
///    `sys.stringTransformations.homographSafeASCII` (a value sent that differs
///    is `DocumentPropertyNotGeneratedError`, 10424). Its `creatorRefersTo` finds
///    the creator's own `preorder` of the name, through `findBy` `$ownerId` and
///    `nameHash` (the sha256d of the normalized label, `"."` and the normalized
///    parent), from an earlier block (`minimumAgeBlocks: 1`), and deletes it
///    (`consume`, beside the `where` entry `$ownerId`); not found is 40120, too
///    young 40142. Its `chooseFirst` refuses the create while another identity
///    holds an unexpired preorder of the name from an earlier block that holds,
///    at `ownerNameHash`, the sha256d of its own owner's id and the name
///    (`ReferencedDocumentPrecededError`, 40149), and the found preorder must
///    hold that hash with the creator's id (else 40120). So a preorder copied
///    from a pending one counts for nobody, and an identity that learns a name
///    from a pending reveal and preorders it itself comes after the revealer.
///    A preorder carries no salt: it holds both hashes, is unique per owner and
///    name (`ownerAndNameHash` over `$ownerId` and `nameHash`, replacing
///    `saltedHash`), is indexed by `nameHash` and creation height
///    (`nameHashAndHeight`) and lives an hour (`ttl` 3600, so it refunds nothing
///    when consumed). A name holds at most 1,000 preorders, the most a
///    `chooseFirst` reads (`max_choose_first_candidates`): the preorder's
///    `atMostAThousandPerName` rule counts them through a countable `nameHash`
///    index (note 4), so a reveal always reads every preorder before its own and
///    no copy can hide an older valid one; a preorder past it is refused (10422). The `domain` keeps `preorderSalt` declared, transient and
///    no longer required, so domains written under v1 and v2 decode. The rule
///    `recordsIdentityIsOwner` (`$transferredAt` equal to `$createdAt` implies
///    `records.identity` equal to `$ownerId`) holds a new name's identity record
///    to its owner (10422); a transfer or a purchase in a later block no longer
///    reads it, and one in the block that created the name is refused. The
///    `domain` sets `canBeDeleted: false`; its property positions, transient
///    list and stored encoding are v2's. `transition_to_version_15` re-stores
///    DPNS through `Drive::apply_contract_rebuilding_document_types`: every
///    preorder made before the upgrade is deleted, unrefunded, and the
///    `preorder` type is rebuilt as a contract update creates a type it adds,
///    the layout a chain born at this version has.
///    `DRIVE_ABCI_VALIDATION_VERSIONS_V11` selects `create_domain_data_trigger`
///    2, which keeps only the parent domain checks: a top-level domain only by
///    the contract owner, the parent present, no subdomains under a name, and
///    the parent's subdomain rule. DPNS needs no new data trigger bindings:
///    its `domain` Replace and Delete rejects are never reached, as
///    `documentsMutable` and `canBeDeleted` false refuse both first (10404).
///    (The bindings move to 3 for the token market contract alone, note 9.)
///
/// 4. **`countOf` and `sumOf` match byte array keys**: `CONTRACT_VERSIONS_V7`
///    selects `parse_property_constraints` 1, whose grammar binds a filter key
///    to a byte array property of the document written (or deleted), `{
///    "nameHash": "nameHash" }`, and reads that property as a byte array, and
///    whose registration check admits a byte array key of the counted type bound
///    to a byte array property (never a constant, `$ownerId` or `$id`). The count
///    or sum tree holds the key as `serialize_value_for_key` writes it, so
///    consensus reads the total as it reads any other. `parse_property_constraints`
///    0 (protocol version 14) reads such a binding as an integer and refuses it.
///
/// 5. **Tradeable tokens**: `TokenTradeMode::Tradeable`
///    (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`) lets a token's holders list it and make
///    offers for it, with the token's minimum fill (`min_fill`, from 1 to `i64::MAX`, else
///    `InvalidTokenOrderFillRangeError` 10463 unpaid on contract create and update, and charged
///    on a `TokenConfigUpdate`). Before this version a contract whose token is tradeable, and a
///    `TokenConfigUpdate` that makes a token tradeable, are refused unpaid as not active
///    (`StateTransitionNotActiveError`, 10603), the refusal older software gives by failing to
///    decode the mode. A tradeable token must be transferable (`NonTransferableTokenTradeableError`,
///    10281): contract create and update refuse it unpaid, and the batch `advanced_structure` 2
///    (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) checks the trade mode an update proposes against the
///    contract the action carries, charged, so check_tx refuses the update too. Protocol v14 and
///    earlier keep the batch `advanced_structure` 1.
///
/// 6. **Documents checked natively**: `CONTRACT_VERSIONS_V7` selects
///    `validate_document` 2, which checks a document against its type's typed
///    validation schema (`document_type::schema::validation_schema`, built by
///    `validation_schema` 0) instead of the `jsonschema` crate, reaching the same verdicts and error codes but for
///    one rule of its own: in a `uniqueItems` list, `0.0` and `-0.0` are one
///    value however long the list. A document type whose schema the validation
///    schema does not represent has its documents refused
///    (`InvalidContractStructure`): no document is checked through the
///    `jsonschema` crate. Protocol v14 and earlier keep `validate_document` 1.
///
/// 7. **Schemas checked natively at registration**: `CONTRACT_VERSIONS_V7`
///    selects `check_document_type_schema` 1: full validation checks a
///    document type's schema with the rules of document meta-schema v3 in
///    Platform's own code instead of through the `jsonschema` crate, refusing
///    with the same `JsonSchemaError`. It holds an index property name to 255
///    bytes, where the meta-schema admitted 256 characters, treats `0.0` and
///    `-0.0` as one value in a list that must be unique, such as an `enum`'s
///    members, however long the list, as the document check of note 6 does,
///    and refuses a document type the typed validation schema does not represent
///    (`InvalidContractStructure`), so every type registered from this version
///    has its documents checked natively (note 6). The parser is generation 3
///    at both versions, and loading a stored contract runs no schema check.
///    Protocol v14 and earlier keep `check_document_type_schema` 0.
///
/// 8. **A token's decimals, generated**: `CONTRACT_VERSIONS_V7` selects
///    `apply_generated_from` 1, which admits the `generatedFrom` function
///    `sys.tokens.decimals` on an integer property admitting 0 to 255: the
///    decimals of the token an identifier property names, read from the
///    token's contract. It is the first function that reads state, so its
///    value is generated when a document is created and kept after, unless a
///    replace names another token: the document create and replace
///    transformers write a left-out value, or check a sent one, from state or
///    from the stored document (`generate_state_read_properties`), billing the
///    reads, and refuse a token that does not exist
///    (`ReferencedEntityNotFoundError`) or a sent value other than the one
///    generated or kept (`DocumentPropertyNotGeneratedError`). Registration
///    refuses it on an indexOnly type and in a contested index; clients leave
///    the property out, and a proof verifier takes it from the proved
///    document. Those call sites were extended in place and are inert before
///    this version, where no type declares such a function. Protocol v14 and
///    earlier keep `apply_generated_from` 0.
///
/// 9. **The token marketplace's state**: `DRIVE_VERSION_V10` creates the two escrow trees at
///    genesis (`create_initial_state_structure` 5) and `transition_to_version_15` inserts them
///    through the same helper: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, a BigSumTree of one SumTree
///    per token of the tokens its open listings hold, and `[PreFundedSpecializedBalances,
///    PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`, a SumTree of the credits open offers hold. Its
///    `calculate_total_tokens_balance` 2 counts the escrowed tokens on the balance side of the
///    token conservation check; the credit check already counts every prefunded specialized
///    balance. The escrow methods (`order_escrow`) are `None` in every earlier table. The offers'
///    escrow key sits two levels into the prefunded specialized balances layer, below the keys
///    the voting balance and contract fee pot estimations describe, so their v2 and v1 keep a
///    deeper description of that layer another write of the batch registered; and
///    `apply_drive_operations` 2 merges each operation's estimated layer descriptions into the
///    batch's as one covering both (the deeper depth, with the nodes priced dearer there), so an
///    estimate does not depend on the operations' order. That raises the estimated fee of any
///    batch whose operations describe a shared layer differently, an ordinary document
///    transition included (its nonce update describes the root a level deeper than the
///    document); the fee a transition pays is unchanged.
///    `SYSTEM_DATA_CONTRACT_VERSIONS_V4` selects the token market contract v1, whose
///    `ask` and `bid` types hold the open orders, each with the token's decimals when it was
///    placed, which prices its fills and keys its `book` and `depth` index entries
///    (`tokenId`, `decimals`, `price`) so prices of different scales never mix: registered at
///    genesis from this version and
///    stored without storage flags by `transition_to_version_15`. Users cannot write them:
///    `creationRestrictionMode` 2 refuses a create, and the data trigger `bindings` 3
///    (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuse a replace or a delete.
///
/// 10. **Token listings**: three token transitions of a batch, `TokenPlaceAsk` (lists an amount
///    of a tradeable token at a price in credits per whole token and moves it into escrow),
///    `TokenFillAsk` (buys part or all of a listing, paying the seller the change in the
///    listing's rounded-up running total) and `TokenCancelOrder` (returns what an order holds
///    to its owner). The batch's `is_allowed` gate refuses each, unpaid, before this version,
///    as software that cannot decode them refuses them. The checks that read nothing beyond the
///    contract (a tradeable token, an amount of at least its minimum fill, what the listing is
///    worth at its decimals: at most `i64::MAX` credits, `InvalidTokenOrderValueError` 10465) run
///    with the batch `advanced_structure` 2, so check_tx refuses them too; the others with state
///    validation. A listing's amount and price (1 to `i64::MAX`) are checked with the
///    transition's structure, unpaid. A listing names no decimals and no minimum fill: it records
///    the token's decimals as the token has them when it is placed (the token market contract's
///    `generatedFrom` `sys.tokens.decimals`, note 8), and its fills are priced at them; the
///    token's minimum fill (`TokenTradeMode::Tradeable`'s `min_fill`, from 1 to `i64::MAX`) is
///    read by every fill as the token has it then, and a listing of less is refused
///    (`InvalidTokenOrderFillRangeError` 10463). A listing of a token its contract does not
///    define bumps the nonce with the token base
///    rules' error. An order of a token with 39 decimals or more (decimals are a `u8`, and a
///    `Conventions` update is not held to the 16 a contract is) has a whole token of more base
///    units than any amount times any price: it is worth one credit rounded up, so a listing's
///    first fill pays 1 and the rest 0, and nothing rounded down. Their structure and state
///    validations and their high-level operation converters are 0 in every table, which only this
///    version reaches.
///
/// 11. **Token offers**: `TokenPlaceBid` (offers credits per whole token for an amount of a
///     tradeable token, moving what the offer is worth, rounded up, from the buyer's balance into
///     the offer's escrow) and `TokenFillBid` (sells part or all of the amount still wanted, paid
///     the change in the offer's running total rounded down out of the escrow); a
///     `TokenCancelOrder` of the `bid` side returns the escrow. Refused before this version by the
///     same `is_allowed` gate. An offer's terms are checked as a listing's are; it signs the
///     decimals its price is at, is refused unless they are the token's
///     (`TokenOrderDecimalsMismatchError`, with the batch `advanced_structure`, as a listing's), and records them: its
///     escrow and every sale into it are priced at them. Their
///     validations and converters are 0 in every table.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: create_initial_state_structure 5 adds the token marketplace escrow trees; calculate_total_tokens_balance 2 counts escrowed tokens; the escrow primitives; the identity balance, token balance, prefunded specialized balance and contract fee pot estimations (for_balances 1, for_token_balances 1, estimated_cost_for_prefunded_specialized_balance_update 2, add_estimation_costs_for_contract_fee_pot_update 1) keep a deeper layer description; apply_drive_operations 2 merges each operation's layer descriptions (note 9)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: verify_chain_lock_locally 1 rejects a chain lock locally only when Platform holds the quorums active at its signing height, and verify_chain_lock_through_core 1 has Core's verifychainlock check the others before submitchainlock
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: identity update advanced_structure 1 refuses unpaid an identity update whose added key's proof of possession does not verify (note 2); create_domain_data_trigger 2 (note 3); data trigger bindings 3 refuse a replace or a delete of a token market order (note 9)
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
        contract_versions: CONTRACT_VERSIONS_V7, // changed: parse_property_constraints 1 lets countOf and sumOf match byte array keys (note 4); validate_document 2 checks documents natively (note 6); check_document_type_schema 1 checks schemas natively at registration (note 7); apply_generated_from 1 admits sys.tokens.decimals (note 8)
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V3,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V4, // changed: DPNS v3 checks a name's create with keywords (note 3); the token market contract is selected (note 9)
    fee_version: FEE_VERSION3,
    system_limits: SYSTEM_LIMITS_V4,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
