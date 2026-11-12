use crate::version::consensus_versions::{ConsensusEvidenceParams, ConsensusVersions};
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
use crate::version::dpp_versions::dpp_state_transition_versions::v5::STATE_TRANSITION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_token_versions::v3::TOKEN_VERSIONS_V3;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v3::VOTING_VERSION_V3;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v2::DRIVE_ABCI_STRUCTURE_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v11::DRIVE_ABCI_VALIDATION_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::fee::v4::FEE_VERSION4;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v4::SYSTEM_DATA_CONTRACT_VERSIONS_V4;
use crate::version::system_limits::v5::SYSTEM_LIMITS_V5;
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
///    (The bindings move to 3 for the token market contract, note 17, and
///    DashPay, note 24, not for DPNS.)
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
///    offers for it. Before this version a contract whose token is tradeable, and a
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
///    two rules of its own. First, in a `uniqueItems` list, `0.0` and `-0.0` are
///    one value however long the list. Second, a `format: regex` value is
///    checked for its syntax alone: whether it parses as a regular expression,
///    not whether it compiles, since no consensus path uses the value as a
///    pattern. This accepts a value that parses but that the engine does not
///    compile, and one that names a Unicode property the engine holds no
///    character class for; a value that is not valid regular-expression syntax
///    is still refused. A document type whose schema the validation
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
///    refuses a document type schema, or the contract's `$defs`, naming a key
///    twice in any object it holds, at its root or below it, as sent, whose
///    first entry the parser may read where every check reads the last
///    (`InvalidContractStructure`; a stored contract with one, which no JSON
///    could have written, must drop it to be updated),
///    refuses a contested index whose contest its index values do not decide
///    (`InvalidContractStructure`, note 19) or that is keyed by a property path or a
///    system property (`InvalidContractStructure`, note 21),
///    and refuses a document type the typed validation schema does not represent
///    (`InvalidContractStructure`), so every type registered from this version
///    has its documents checked natively (note 6). It also selects
///    `validate_max_depth` 2, the depth and cycle check run at registration and
///    update, which reads a `$ref` pointer by RFC 6901, as the validators do,
///    where generation 1 read it as a `.`-separated path: so a `$ref` the two
///    readings resolve differently, through a key holding a `.`, a `~0`/`~1`
///    escape or a percent escape, is refused here
///    (`DataContractMaxDepthExceedError` or `InvalidJsonSchemaRefError`), the
///    check now seeing the schema the validators walk; a `$ref` pointer holding
///    none of those resolves and is measured as before. It also holds an array
///    to the maximum depth as it does a map, where generation 1 checked only
///    maps, so a schema whose array nesting alone exceeds the maximum depth,
///    such as a deeply nested `const` or `enum` value, is now refused at
///    registration and update (`DataContractMaxDepthExceedError`). The parser
///    is generation 3 at both versions, and loading a stored contract runs no
///    schema check.
///    Protocol v14 and earlier keep `check_document_type_schema` 0 and
///    `validate_max_depth` 1.
///
/// 8. **Contracts stored compiled**: `CONTRACT_VERSIONS_V7` selects
///    `contract_storage_serialization_version` 2: Drive stores a contract it creates or
///    updates in format version 2, its schemas with each document type compiled
///    (`DocumentTypeInSerializationFormat` 0), and reads such a contract back without parsing
///    its schemas, so it keeps the meaning it was compiled with. State transitions, unproved
///    query responses and JSON keep format version 1 (`contract_serialization_version`), and
///    a transition carrying format version 2 is refused while decoding at every version. The
///    65,000-byte cap still applies to the contract's format version 1 bytes; the compiled
///    bytes, about 1.8 times as many, are paid by the byte, and a contract read pays its
///    larger size. `DRIVE_VERSION_V10` selects `apply_contract_with_serialization` 1, which
///    writes nothing when a contract's format version 1 bytes match the stored one's.
///    Contracts stored before this version stay in their format until their next write, a
///    contract update or a token configuration update, which re-stores them compiled: its
///    sender pays the growth, for a token configuration update the group member whose action
///    closes it, not the owner. A chain started at this version stores every system contract
///    compiled at genesis, while a chain upgraded to it keeps them in the format they were
///    stored in, so a transition reading one pays more on the first. A proof of a contract
///    stored compiled carries the compiled bytes, about 1.8 times as large, and clients need a
///    decoder that reads format version 2 to verify it. A contract compiled takes at most
///    `max_compiled_contract_size` (1 MiB): registration and update refuse a contract over
///    it, and a token configuration update one it would take over it
///    (`DataContractCompiledMaxSizeExceededError`, 10282). check_tx and recheck, which run
///    no state validation, hold a contract update to it through
///    `DRIVE_ABCI_VALIDATION_VERSIONS_V11`'s `transform_into_action` 1, with the contract's
///    time fields at their widest: a bound on what a block measures.
///    Protocol v14 and earlier keep format version 1 (0 before v9) and
///    `apply_contract_with_serialization` 0.
///
/// 9. **Token configuration updates on read-only contracts**: a token's configuration is part
///    of its contract, so a performed `TokenConfigUpdate` stores the contract again, which Drive
///    refuses for a read-only contract. No validation step looked at the flag, so such an update
///    passed validation and check_tx and failed only when its block was executed, which leaves
///    it out of the block. The batch `advanced_structure` 2
///    (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuses an update of a token on a read-only
///    contract, performed or a group member's vote, with `DataContractIsReadonlyError` (40001),
///    charged, so check_tx refuses it too. Protocol v14 and earlier keep the batch
///    `advanced_structure` 1.
///
/// 10. **Yes/no vote polls**: a second kind of masternode vote poll, which asks
///     "should this happen?" and passes on a supermajority of yes over yes plus
///     no with a minimum of voting power cast. A poll is identified by its purpose
///     (a decision proposal's document id): its id is derived from the purpose
///     alone, a vote names the poll by that id, and the rule is kept in the poll's
///     stored info. A minimum given as a share of the network's voting power
///     leaves PoSe-banned masternodes out of the total. A masternode vote naming a
///     yes/no poll is active from this version (`StateTransition::active_version_range`);
///     the polls live under the votes tree's decisions branch, which genesis
///     (`create_initial_state_structure` 4 behind
///     `>= YES_NO_VOTE_POLL_INITIAL_PROTOCOL_VERSION`, 15) and
///     `transition_to_version_15` create. Masternode vote `transform_into_action`
///     1 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) looks up the previous answer, and
///     `DRIVE_VERSION_V10` registers and sweeps the votes. Opening a yes/no poll
///     (`Drive::open_yes_no_vote_poll`; every `yes_no` drive method is `Some(0)`) is active
///     from this version, for an end date from `min_yes_no_vote_poll_duration_ms` (one week)
///     to `max_yes_no_vote_poll_duration_ms` (four weeks) after the opening block
///     (`YesNoVotePoll::end_date_window`), and its end date entry goes through
///     `add_vote_poll_end_date_query_operations` 1, which adds only the entry when
///     the batch already queues the end date's tree with other storage flags.
///     Every other yes/no method is active from this version too and `None`
///     (not active) in every earlier table: the poll's id and rules (`VOTING_VERSION_V3`),
///     the end-of-block tally, record and clean-up (`DRIVE_ABCI_METHOD_VERSIONS_V11`
///     voting), removing finished polls from the end date index
///     (`remove_vote_poll_end_date_query_operations`) and the proof verifiers
///     (`DRIVE_VERIFY_METHOD_VERSIONS_V4`).
///     A poll's pass threshold is at least `min_yes_no_vote_poll_supermajority_percent` (50,
///     a simple majority) of yes plus no. Decision proposals (note 12) open the polls.
///
/// 11. **A consume deletes what its lookup read**: document reference validation 1
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) keeps the commitment that the `refersTo` lookup of
///     a reference declaring `consume` finds in the `ConsumedDocument`; every lookup reads as
///     document reference validation 0's does. The create
///     conversion 1 (`DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5`, which `DRIVE_VERSION_V10`
///     selects) deletes the document from it
///     (`Drive::force_delete_read_document_for_contract_operations`, the force delete's
///     operations less its read), where conversion 0 deletes it by id and bills a second read.
///     A consume costs one document read fewer. What that read cost depends on where the
///     commitment sits in its tree, so the drop differs from create to create (10,660 credits
///     for the DPNS v3 name create the tests pin). Storage fees and refunds are unchanged.
///     Protocol v14 and earlier keep document reference validation 0 and the create
///     conversion 0.
///
/// 12. **Decision proposals**: the decision proposals system contract
///     (`SystemDataContract::DecisionProposals`, `SYSTEM_DATA_CONTRACT_VERSIONS_V4`'s
///     `decision_proposals` 1) is registered at genesis from this version (`create_genesis_state`
///     1, `>= 15`) and inserted by `transition_to_version_15`. Creating its `proposal` opens a
///     yes/no poll (note 10) whose purpose is the proposal and whose rule the proposal states:
///     the create states the decision proposal fund (`FEE_VERSION4`'s
///     `decision_proposal_fund_required_amount`, 0.5 Dash) as its prefunded voting balance,
///     under the name `proposal`. Document create structure validation 2
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuses any other fund, a rule outside the limits
///     (`DecisionProposalVotePoll::from_proposal`, `VOTING_VERSION_V3`'s
///     `decision_proposal_vote_poll`) and an end date outside one to four weeks after the block,
///     so check_tx refuses what a block would; the create conversion 1 reads the poll from the
///     proposal,
///     moves the fund from the proposer to the poll's prefunded balance and opens the poll
///     (`DriveOperation::VotePollOperation`, which estimates the opening's costs too, as
///     check_tx's fee estimate converts a create no state validation has seen). Signaling
///     only: nothing acts on a result. A proposal's rule never changes (`immutable`); its
///     owner may edit its title, description and url for `min(1 day, run / 5)` after creating
///     it (conditional `immutable` entries), and every version is kept (`documentsKeepHistory`).
///     Protocol v14 and earlier have no such contract.
///
/// 13. **Private document types**: `CONTRACT_VERSIONS_V7` selects `parse_private_document` 0:
///     a document type may declare `private` (`{ "entrySize": n }`), which the native schema
///     check of note 7 admits where that version is set, and meta-schema v3 refuses. Its
///     documents are fixed-size encrypted entries, which the chain never reads. Every parse
///     holds such a type to the keywords a private type may declare (no index, history,
///     transfer, trade, cost, rule or reference; documents neither mutable nor deletable; no
///     system field; no elected moderation), and registration holds `entrySize` to
///     `max_private_document_entry_size`, 4096 bytes in `SYSTEM_LIMITS_V5`. The compiled
///     stored form of note 8 records the declaration. A contract update cannot add, remove or
///     change it (`DocumentTypeUpdateError`), and may only add properties after a private type's
///     existing ones, at the end of the type and of each object, since its immutable entries are
///     read with the type as it is now (`DocumentTypeUpdateError` too). No reference may name a private type: a
///     `findBy`, an `inList` list, a `countOf` or a `sumOf` naming one of the same contract is
///     refused by the contract parse (`InvalidContractStructure`), and every other reference,
///     to a type of the same contract or another, by contract registration
///     (`ReferencedDocumentTypePrivateError`, 40150). A Batch refuses every document
///     transition on a private type (`DocumentTypePrivateError`, 10425) as soon as it
///     resolves the type, before the contract moderation gate. Protocol v14 and earlier keep
///     `parse_private_document` `None`: their parse ignores the keyword, registration refuses
///     it from protocol version 12 on (meta-schemas v1 to v3 close the doctype level), and no
///     stored contract carries it (the protocol version 12 migration stripped unknown
///     doctype-level keys from the contracts admitted under meta-schema v0).
///     Drive keeps a private type's entries in a GroveDB private document store at the
///     type's key `[0]`, created with the contract or the update adding the type, in place
///     of the primary-key tree (`insert_empty_private_document_store_operation` 0,
///     `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`, `None` before); each stored entry is the client's behind a 16-byte header.
///     The store's chunk power is the largest keeping a compacted chunk near
///     `private_document_store_target_chunk_bytes` (1 MiB), bounded by
///     `private_document_store_min_chunk_power` and `private_document_store_max_chunk_power`
///     (4 and 11), all in `SYSTEM_LIMITS_V5`; it is committed into the store. Document
///     queries refuse a private type; `getPrivateDocumentEntries` reads and proves a page of
///     its store, from a position to the end of the chunk holding it, with the chunk power the
///     store committed (`fetch_private_document_entries` and `prove_private_document_entries`
///     `Some(0)` in `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`, `verify_private_document_entries`
///     `Some(0)` in `DRIVE_VERIFY_METHOD_VERSIONS_V4`, `None` before).
///
/// 14. **A batch's fee estimate no longer depends on the order of its operations**:
///     `apply_drive_operations` 2 (`DRIVE_VERSION_V10`) resolves an estimated batch's layer
///     descriptions once the whole batch is lowered (`EstimatedLayers::resolve`). Each layer a
///     batch's writes describe more than one way, the nested estimators of one operation
///     included, is described by the costliest of them (`costliest_layer`): the deepest depth any
///     of them has, and at that depth the nodes GroveDB's `average_case_merk_propagate` prices
///     dearest, each description priced at that depth (so the kept description may pair one
///     write's nodes with another's depth), among the descriptions of subtrees when a write
///     describes the layer as holding subtrees, then a fixed tie-break. The root is also kept at
///     the level of the deepest root key the batch writes under, as the root's shape records it,
///     as some estimators name a shallower one (the credit shielded pool's level 3 for
///     `ShieldedBalances` and the contract groups' level 3 for `ContractGroups`, both at level 4;
///     the identity nonce's and revision's level 0 for `Identities` at level 1).
///
///     Generation 1 priced every layer with the description written last, so the order mattered:
///     an identity contract nonce update (the root at level 1) built after a prefunded
///     specialized balance write (the root at level 3) lowered the estimate of both, and a token
///     mint, whose balance estimator describes the root at level 2 after its supply estimator
///     described it at level 3, was priced below its supply's description. Two estimators write
///     a layer only when nothing described it yet (the contract fee pot's root, at level 3, and
///     the contract other tree's contract root): generation 1 kept the earlier description, so
///     an action fee's pot write, which comes after the transition's own operations, was priced
///     at the level 1 its identity balance write gave the root. The estimate rises for a batch
///     whose priced description was not the costliest: an ordinary document transition (the
///     root at the nonce's level 1, not the document's 0), a document transition paying an
///     action fee (the root at the pot's level 3, not 1), a token mint, and a transition writing
///     to the credit shielded pool (the root at level 4, not 3; token pools sit under `Tokens`).
///
///     Four estimators describe their layers as they are from this version:
///     `remove_reference_for_index_level_for_contract_operations` 2 describes an index's entries
///     for a value (its `[0]` layer) as references, as the insert does, where 1 described them as
///     subtrees, so a create consuming a document of its own type under the same index value
///     was priced with the cheaper description; under a summable index it describes them, and
///     the reference it deletes, as references with sum items, as
///     `add_reference_for_index_level_for_contract_operations` 1 describes the entries an insert
///     adds to, where 0 described plain references. GroveDB prices a reference with a sum item
///     10 bytes dearer per node, so a document create or delete under summable indexes is
///     estimated 138,600 credits higher per summable index whose entries it writes (the entries
///     layer's 33 estimated nodes, at 420 credits a byte), and a delete 200 more for reading the
///     deleted reference: the tip jar fixture's tip, under three summable indexes, from
///     28,499,880 to 28,915,680 to create and from 27,754,220 to 28,170,620 to delete.
///     `stateless_delete_of_non_tree_for_costs` 1 gives each height of a delete's climb the
///     layer GroveDB deletes in there, where 0 gave the one above it. And
///     `for_total_system_credits_update` 1 describes `Misc` with its expirations and token
///     supplies trees beside the credits item, where 0 described the credits item alone, which
///     generation 1 could not price after a token mint in the same batch. GroveDB prices
///     `Misc`'s subtree nodes cheaper than the credits item's, so this one estimate falls:
///     every batch that adds to or removes from the total system credits (an asset lock
///     funding, a withdrawal, a move between an identity and the shielded pool) is estimated
///     7,980 credits lower (19 replaced bytes at 420 credits a byte), still above what it
///     costs. A lone addition of 5 credits is estimated at 332,800 at protocol v14 and 324,820
///     here, and costs 265,200 at both.
///
///     The estimate is consensus: `validate_fees_of_event` checks a payer's balance against it,
///     decides whether a gas sponsor or the signer pays (a strict sponsorship is refused), checks
///     a budgeted signing key's remaining budget against its user fee increase (refusing the
///     transition, unpaid, with `IdentityPublicKeyBudgetExceededError`), and checks a shield's
///     pool fee and a shielded spend's denomination against it; check_tx reports it as
///     `gas_wanted`. A raised estimate can so refuse what protocol v14 admitted, and the lower
///     system credits estimate admit what it refused near a balance or budget threshold. The fee a
///     transition pays is unchanged, as applying a batch is generation 1's. Protocol v14 keeps
///     `apply_drive_operations` 1 and the estimators' earlier versions, and protocol v13 and
///     earlier keep `apply_drive_operations` 0; neither resolves an estimate.
///
/// 15. **A token's decimals, generated**: `CONTRACT_VERSIONS_V7` selects
///     `apply_generated_from` 1, which admits the `generatedFrom` function
///     `sys.tokens.decimals` on an integer property admitting 0 to 255: the
///     decimals of the token an identifier property names, read from the
///     token's contract. It is the first function that reads state, so its
///     value is generated when a document is created and kept after, unless a
///     replace names another token: the document create and replace
///     transformers write a left-out value, or check a sent one, from state or
///     from the stored document (`generate_state_read_properties`), billing the
///     reads, and refuse a token that does not exist
///     (`ReferencedEntityNotFoundError`) or a sent value other than the one
///     generated or kept (`DocumentPropertyNotGeneratedError`). Registration
///     refuses it on an indexOnly type and in a contested index; clients leave
///     the property out, and a proof verifier takes it from the proved
///     document. Those call sites were extended in place and are inert before
///     this version, where no type declares such a function. Protocol v14 and
///     earlier keep `apply_generated_from` 0.
///
/// 16. **Zero consensus evidence budget**: consensus parameter update 2 installs
///     the complete evidence policy at activation, including a zero byte budget.
///     Protocol v14 and earlier retain their existing evidence limits.
///
/// 17. **The token marketplace's state**: `DRIVE_VERSION_V10` creates the two escrow trees at
///     genesis (`create_initial_state_structure` 5) and `transition_to_version_15` inserts them
///     through the same helper: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, a BigSumTree of one SumTree
///     per token of the tokens its open listings hold, and `[PreFundedSpecializedBalances,
///     PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`, a SumTree of the credits open offers hold. Its
///     `calculate_total_tokens_balance` 2 counts the escrowed tokens on the balance side of the
///     token conservation check; the credit check already counts every prefunded specialized
///     balance. The escrow methods (`order_escrow`) are `None` in every earlier table. The offers'
///     escrow key sits two levels into the prefunded specialized balances layer, below the keys
///     the voting balance and contract fee pot estimations describe, so their v2 and v1 keep a
///     deeper description of that layer another write of the batch registered, and
///     `apply_drive_operations` 2 (note 14) keeps it whatever the order of the batch's writes.
///     `SYSTEM_DATA_CONTRACT_VERSIONS_V4` selects the token market contract v1, whose
///     `ask` and `bid` types hold the open orders, each with the token's decimals when it was
///     placed, which prices its fills and keys its `book` and `depth` index entries
///     (`tokenId`, `decimals`, `price`) so prices of different scales never mix: registered at
///     genesis from this version and
///     stored without storage flags by `transition_to_version_15`. Users cannot write them:
///     `creationRestrictionMode` 2 refuses a create, and the data trigger `bindings` 3
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuse a replace or a delete.
///
/// 18. **Core fork activations recorded in state**: `update_core_info` 1
///     (`DRIVE_ABCI_METHOD_VERSIONS_V11`) asks Core, in a block that moves the chain locked
///     Core height, whether Dash Core's V24 is active at that height: the coinbase of that
///     Core block has payload version 4 or higher, which Core requires exactly while V24 is
///     active. The first block that sees it active stores the height in `Misc`
///     (`CORE_FORK_V24_ACTIVATION_KEY`, a big endian `u32` written once, through
///     `DRIVE_VERSION_V10`'s `platform_system` methods), where the app hash covers it, and
///     `update_core_info` does not ask Core about V24 again. The masternode list's operator
///     key normalization reads the record too (`update_masternode_list` 2, note 32), where
///     `update_state_masternode_list` v0 and v1 ask Core at the block's chain locked height
///     when Core re-reports an operator key unchanged or re-encoded. A chain born at this
///     version asks at init chain, at the genesis core height. A block that keeps the chain
///     locked height asks nothing, so a chain upgraded to this version records V24 at its
///     first block that moves the height to one where V24 is active. The recorded height is
///     the chain locked height at which Platform first observed V24 active, not Core's
///     activation height, and it can trail that height without bound: on a chain where V24
///     was active long before the upgrade, it is the chain locked height of that first block.
///     A Core error or a coinbase without a payload fails the block, as other Core errors in
///     `update_core_info` do. Protocol v14 and earlier keep `update_core_info` 0, which
///     neither asks nor records.
///
/// 19. **A contest is decided by its index values**: a document joins its type's contest
///     when every `fieldMatches` entry matches it, and two documents with the same index
///     values share one unique index entry. A `fieldMatches` field outside the index, or a
///     match telling `""` from `"\0"` (which share an index key) on a string that may be
///     empty, lets a document outside the contest take the index values of one in it, and
///     the award at the end of the contest then finds the entry taken. Registration under
///     `check_document_type_schema` 1 (note 7) refuses such a contest
///     (`Index::contest_is_decided_by_index_values`, `InvalidContractStructure`),
///     and document create state validation 3 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`)
///     refuses a create outside the contest of a type registered earlier with one while a
///     contest for the same index keys is live (`DuplicateUniqueIndexError`), reading the
///     poll at its path, which encodes the values as index keys, so a create of `"\0"` finds
///     the contest of `""`. Only such a type pays that lookup, and not for a document
///     with an empty index key (a missing value, or an empty byte array), whose unique index
///     entry Drive keeps per document (`unique_index_entry_is_shared`), nor for an index
///     holding a system property (`$ownerId`), whose poll no create can address. A contract
///     registered earlier with such a contest keeps it, but an update, which carries the
///     whole contract, is refused like a registration. Protocol v14 and earlier keep
///     `check_document_type_schema` 0 and document create state validation 2.
///
/// 20. **Asset unlocks with a stable txid once Core's V24 is active**:
///     `build_untied_withdrawal_transactions_from_documents` 1
///     (`DRIVE_ABCI_METHOD_VERSIONS_V11`) gives a withdrawal it pools asset unlock payload
///     version 2 once Platform has recorded Dash Core's V24 as active (note 18), read in the
///     block's transaction, and the withdrawal's whole amount is not below Core's dust
///     threshold for its output script (`core_dust_relay_fee_per_kb`); otherwise version 1.
///     The txid of a version 2 asset unlock leaves out the requested height, quorum hash and
///     signature, so a withdrawal re-signed after it expires keeps its txid and Core can
///     InstantSend-lock it before it is mined. The version is kept in the untied transaction
///     bytes the queue stores, so withdrawals pooled before the switch stay version 1 when
///     they are re-signed, and a dust withdrawal, the only kind the expiry step marks FAILED,
///     is never version 2. Pooling does not ask Core: a withdrawal pooled in the block that
///     records V24 is already version 2, and until a block records it, the first that moves
///     the chain locked height to one where V24 is active, withdrawals stay version 1, which
///     Core always accepts. Protocol v14 and earlier keep
///     `build_untied_withdrawal_transactions_from_documents` 0, which emits version 1.
///
/// 21. **A contest is keyed by top-level properties**: a contested create keys its vote poll
///     by looking each index property up by name among the document's top-level properties
///     (`Index::extract_values`), where a property path (`records.identity`) or a system
///     property (`$ownerId`) is never found, so the poll holds a null value and Drive fails
///     every contested create of the type without storing it: the contest can never start.
///     Registration under `check_document_type_schema` 1 (note 7) refuses a contested index
///     holding one (`Index::contest_is_keyed_by_top_level_properties`,
///     `InvalidContractStructure`). A contract registered earlier with one keeps it, but an
///     update is refused like a registration. Protocol v14 and earlier keep
///     `check_document_type_schema` 0.
///
/// 22. **A masternode that returns to a voting address keeps its vote**: each voting address
///     a masternode uses has its own voter identity, and a vote must be signed by the key of
///     the identity for the masternode's current address. When the address changes,
///     `update_voter_identity` disables the key of the identity it leaves and, when the
///     identity for the new address already exists because the masternode used that address
///     before, re-enables a key. Version 0 re-enabled it on the identity it had just disabled,
///     so the returning key stayed disabled, every vote of the masternode was refused
///     (`PublicKeyIsDisabledError`), and the key of the address it left stayed enabled,
///     though it cannot vote for the masternode. Version 1 (`DRIVE_ABCI_METHOD_VERSIONS_V11`)
///     re-enables the key of the returning identity. It applies to voting address changes from
///     this protocol version on and repairs nothing at activation: a masternode whose returning
///     key version 0 left disabled cannot vote until its voting address changes again, and
///     that change leaves the key of the identity it moves to enabled, as for any other
///     masternode. Protocol v14 and earlier keep `update_voter_identity` 0.
///
/// 23. **An identity create's identity id is checked in basic structure**: the identity create
///     `basic_structure` 1 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) refuses, without a fee, an
///     identity id other than the one the asset lock's outpoint derives, before the asset lock
///     proof is verified; `advanced_structure` 2 drops that check, which advanced structure 1
///     made after the asset lock proof, and still charges its double SHA-256, so fees do not
///     change. Only the error a block reports for a transition with a wrong identity id and an
///     asset lock that is spent or whose proof does not verify changes; an asset lock proof
///     whose structure is invalid is still refused first, for its structure. Such a transition
///     is refused unpaid either way. Protocol v14 and earlier keep `basic_structure` 0 and
///     `advanced_structure` 0 or 1.
///
/// 24. **DashPay v3 states what its data triggers checked and its ciphertexts' shape, and adds
///     private contact requests**:
///     `SYSTEM_DATA_CONTRACT_VERSIONS_V4` selects DashPay contract v3, loaded at
///     genesis and re-stored by `transition_to_version_15` (`apply_contract`, as the
///     upgrade to v2 did). A `contactRequest`'s `toUserId` refers to an identity
///     (`refersTo`, a missing one is `ReferencedEntityNotFoundError`, 40120) other than
///     the writer (`distinctFrom` `$ownerId`, `DocumentPropertyNotDistinctError`,
///     10419); `encryptedPublicKey` and `encryptedAccountLabel` declare
///     `ecdh-secp256k1-aes256-cbc` between the `senderKeyIndex` and `recipientKeyIndex`
///     keys (`encryptedFor`), so each must be an IV plus whole AES blocks
///     (`InvalidEncryptedPropertyShapeError`, 10420), a check new at this version
///     (protocol v14 accepts any account label of 48 to 80 bytes), and both key indexes
///     gain the u32 `maximum` that requires. A `profile`'s rules `corePaymentAddressType` and
///     `platformPaymentAddressType` hold the first byte of each DIP-33 payment
///     address to 0x00 or 0x01 (`byteAt`, `DocumentPropertyConstraintViolatedError`,
///     10422), and the two address descriptions name the rules instead of the data
///     trigger. Every property is stored as v2 stores it (DashPay keeps sized integer
///     types off, so the key indexes stay i64), so documents written under v1 and v2
///     read back unchanged. The data trigger `bindings` 3 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`,
///     which also add note 17's order rejects) drop the DashPay contact request create and
///     profile create and replace triggers, which made the identity, writer and type byte
///     checks and refused with `DataTriggerConditionError` (40500). Protocol v14 keeps
///     DashPay v2 and bindings 2, and earlier versions the contract and bindings they had.
///     DashPay v3 also declares `privateContactRequest`, a private type (note 13) of 589-byte
///     entries whose properties describe a contact request only its recipient can read:
///     `accountReference`, the DIP-15 compact `extendedPublicKey` (69 bytes), and the optional
///     `accountLabel` (at most 63 bytes), `autoAcceptProof` (38 to 102) and `shieldedAddress`
///     (43). At their largest they encode to 290 bytes, which with the private document
///     envelope's 299 fill an entry. Genesis creates its store as a contract insert does, and
///     `transition_to_version_15` as a contract update adding a private type does (note 13);
///     only note 25's transition appends to it.
///
/// 25. **Private document entries**: a new state transition,
///     `PrivateDocumentCreateFromShieldedPool` (type 29), appends entries to the store of a
///     private document type (note 13), paying a fixed fee from the credit shielded pool: an
///     Orchard spend bundle whose value balance is exactly the fee, with no identity and no
///     platform signature. Its sighash binds the contract, the type name and every entry
///     (`private_document_create_from_shielded_pool_extra_sighash_data`, led by the type byte
///     29). Basic structure 0 refuses, unpaid, a transition carrying no entry or more than
///     `max_private_document_entries_per_transition` (16 in `SYSTEM_LIMITS_V5`,
///     `InvalidPrivateDocumentEntryCountError`, 10426), an entry that is empty, over
///     `max_private_document_entry_size` or not the first entry's size
///     (`InvalidPrivateDocumentEntrySizeError`, 10427), and two entries that are the same bytes
///     (`DuplicatePrivateDocumentEntryError`, 10428). The value balance must equal
///     `compute_private_document_create_from_shielded_pool_fee`: the bundle's shielded fee
///     plus, per entry, its bytes and 32 overhead bytes at the storage rate and a flat
///     10,000,000 credits (`FEE_PRIVATE_DOCUMENT_VERSION1` in `FEE_VERSION4`). After the pool
///     checks a shielded transfer makes, the transform refuses, unpaid, a contract that does not
///     exist, a type it does not have (`InvalidDocumentTypeError`), a type that is not private
///     (`DocumentTypeNotPrivateError`, 40151) and entries of another size than the type's
///     `entrySize` (`PrivateDocumentEntrySizeMismatchError`, 40152). Execution records the
///     nullifiers and the change notes, lowers the pool by the fee and appends each entry
///     behind Drive's 16-byte header (`append_private_document_entries_operations` `Some(0)` in
///     `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`, `None` before). The proof of execution proves the
///     spent nullifiers. The transition is not active before this version
///     (`PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION`), as software that
///     predates it cannot decode one: `validate_shielded_proof` 1 and
///     `validate_minimum_shielded_fee` 0, which protocol v14 selects too, gain an arm for it
///     that is unreachable there.
///
/// 26. **GroveDB V5**: `DRIVE_VERSION_V10` selects `GROVE_V5` (protocol v14 keeps `GROVE_V4`,
///     which GroveDB 6.1.1 shipped). Its private-document gates differ from V4, and protocol v15 needs
///     each: a private document store's entries are read and proved by position range, which
///     `getPrivateDocumentEntries` (note 13) is built on (`private_document_store.get_range`
///     and `range_proof` 1); creating a store is estimated as it is applied, its existence check
///     and the hashes deriving its empty root included (`merk_insert_element` 1 in the average
///     and worst case estimators), so a contract create or update adding a private type is
///     estimated no lower than it costs; and a batch may carry the same keyless append more
///     than once (`apply_batch.keyless_append_repeats` 1), which the batch consistency check
///     Drive runs for estimates and partial batches used to refuse. `GROVE_V5` selects `merge` 2,
///     `per_instance_query_offsets` 1, `element.path_query_push` 2, and
///     `proof.count_offset_reference_empty_tree_rows` 1: branch-local count pages, raw
///     parent-tree echoes and faithful merged queries share one ordinary proof.
///     Private document types and their store exist only from this version, so the
///     first two reach no earlier state; a batch repeating a keyless append arises nowhere in
///     Platform (notes are unique, and note 25 refuses identical entries).
///
/// 27. **Refunds at the storing epoch's rate**: `calculate_fee` 1 (`DRIVE_VERSION_V10`) prices
///     the bytes a storage refund returns at the storage rate of the fee schedule of the
///     protocol version that ran the epoch that stored them, read from that epoch's tree,
///     where the epoch's first block records its protocol version. Bytes stored in the block's
///     own epoch are priced at the block's rate. Generation 0 priced every refund at the
///     block's rate. Every schedule so far has the same storage rate, so no refund changes. A
///     removal from an earlier epoch pays once per such epoch for reading its record, the only
///     fee this changes; generation 0 reads nothing.
///
/// 28. **Storage handover**: when a batch updating documents names a new owner on a stored
///     element, the owner it names owns the element afterwards at any size (`DRIVE_VERSION_V10`,
///     `DRIVE_GROVE_METHOD_VERSIONS_V2`). A transfer or a purchase keeps the stored bytes and
///     moves them to the new owner, the references of the indexes it leaves unchanged included
///     (`update_document_for_contract_operations` 2). A gas sponsor who pays is named as the
///     owner of what the batch writes (`execute_event` 2), and when the old or new owner is
///     the owner of a contract whose document type the batch updates offers sponsorship, the
///     old owner is refunded for the whole element and the payer charged for all of it, so a
///     document's owner cannot keep storage the sponsor paid for. A poll's end date entries
///     name whoever the document's flags name
///     (`add_contested_document_for_contract_operations` 2). A moderator's document field
///     change hands the document to nobody: it is written in the name of whoever the stored
///     document's flags name (contract user moderation `transform_into_action` 1,
///     `DRIVE_ABCI_VALIDATION_VERSIONS_V11`), so the moderator pays for the bytes the change
///     adds and a document a sponsor paid for stays the sponsor's.
///
/// 29. **A document that may be contested is looked up and verified**: a single document query
///     in its "maybe contested" status looks among the documents of its type and in the
///     contest's area at once. `verify_proof_keep_serialized` 1
///     (`DRIVE_VERIFY_METHOD_VERSIONS_V4`) verifies its proof against the merged query and
///     answers with the document from whichever place holds it. Generation 0 asked for the
///     absent keys by name, which needs an overall limit the merged query does not have, and
///     so refused every such query. The query states both branch caps itself
///     (`single_document_path_query` 1, `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`), as the merge of
///     `GROVE_V5` (note 26) no longer derives one above a subquery. The ordinary branch follows
///     the current-revision pointer in the composite history layout (note 35). No state
///     transition is proved in this status, so block execution and its proofs are unchanged.
///
/// 30. **A chain asset lock proof is not held to the input cap**: the asset lock transaction
///     an instant proof embeds, with its InstantSend lock, may carry at most
///     `max_asset_lock_transaction_inputs` (100) inputs, so that the proof fits a state
///     transition. `validate_asset_lock_transaction_structure` 0 checked the cap, and Drive
///     runs it on the transaction it fetches from Core for a chain proof too, though a chain
///     proof carries only an outpoint and a height: a mined asset lock of more than 100
///     inputs, which no instant proof may carry either, could not be redeemed at all
///     (`IdentityAssetLockTransactionTooManyInputsError`, 10534). `STATE_TRANSITION_VERSIONS_V5`
///     selects `validate_asset_lock_transaction_structure` 1, which drops the cap, and
///     `validate_instant_asset_lock_proof_structure` 1, which checks it after the instant
///     lock's txid and before the transaction's structure, where 0 reached it, so an instant
///     proof gets the verdict and the error it got before. Both refusals are unpaid. Protocol
///     v14 and earlier keep both at 0, so the blocks committed under them replay with the same
///     verdicts.
///
/// 31. **A single document is looked up at a block time**: in a document type that keeps
///     history, `single_document_path_query` 1 (`DRIVE_DOCUMENT_METHOD_VERSIONS_V5`) selects
///     the newest retained revision at or before the requested time, clamped to `i64::MAX`.
///     With composite history (note 35), it walks time-and-revision keys descending, taking
///     the highest revision at equal times. Before the first retained revision it returns
///     absence. Migrated histories retain the last write at each old timestamp. The legacy
///     layout instead excludes the latest-version pointer below its time keys.
///     `verify_proof_keep_serialized` 1 (`DRIVE_VERIFY_METHOD_VERSIONS_V4`) verifies the whole
///     query or a subset against that same bound. Generation 0 cannot verify a timed lookup.
///     A document with history that may be contested is refused at a block time: a contest's
///     area keeps no history. Explicitly contested queries ignore the time, as do all queries
///     of types without history. These are single-document Rust and wasm-drive-verify reads;
///     documents queries with a block time still refuse history types after activation.
///     No state transition is proved at a block time, so execution proofs are unchanged.
///
/// 32. **Operator payouts follow Dash Core**: `update_masternode_list` 2
///     (`DRIVE_ABCI_METHOD_VERSIONS_V11`) keeps the operator payout of an updated masternode's
///     Core diff, where earlier versions took operator payouts only from full masternode states,
///     so a payout set or changed after Platform first listed a masternode never reached it.
///     Core empties the payout on every operator key change and prints a payout only when it
///     differs between the diff's two Core heights, so for a key change printed without one the
///     payout is read from Core's state of that masternode at the block's chain locked height
///     (`protx info`, one call for each operator key change or revocation of a listed
///     masternode that the diff prints without a payout): the new operator no longer inherits the previous operator's payout key, nor
///     starts without the one it set. A Core error fails the block. An operator key Core only
///     re-encodes is still the stored key, and whether Core's V24 is active is read from the
///     Core fork record (note 18) instead of asked of Core. A diff that does not print the PoSe
///     revived height keeps the stored one. `update_operator_identity` 1 reconciles every
///     TRANSFER key of the operator identity it updates with the payout: the payout's key is
///     kept, re-enabled or added, and every other enabled TRANSFER key disabled, with no
///     fallback to an earlier operator's key, so the same payout applied twice changes nothing
///     the second time. A key change never touches the TRANSFER keys of an identity that already
///     exists, whatever the payout: the identity a masternode leaves keeps them, and so does an
///     identity it switches back to, until that identity's next payout change. Each step of
///     `update_operator_identity` 1 touches only its own kind of key, and a key whose hash
///     another key of the identity has is added like any other: `DRIVE_VERSION_V10`'s
///     `add_new_keys_to_masternode_identity` 1 adds it without a second non-unique key hash
///     reference where version 0 failed the block. To fix what earlier versions left, the first
///     block of this version (`transition_to_version_15`) reads Dash Core's full masternode list
///     at the last committed chain locked core height, the height Platform's list is at then.
///     Where a stored operator payout differs from Core's, it stores Core's, and it reconciles
///     the TRANSFER keys of every masternode's current operator identity with Core's payout
///     through `update_operator_identity` 1. A Core error, or a Core list whose masternodes are
///     not Platform's, fails that block. Protocol v14 and earlier keep
///     `update_masternode_list` 0 or 1, `update_operator_identity` 0 and
///     `add_new_keys_to_masternode_identity` 0.
///
/// 33. **A reference whose target the document names**: `CONTRACT_VERSIONS_V7` selects
///     `apply_property_reference` 1: a `permanentDocument` or `moderatedDocument` `refersTo`
///     on an identifier property may read its `contractId` and its `documentType` from the
///     referring document, each written `"$self.<property>"` for a top-level property of
///     it (an identifier and a string), with `documentTypeRequirements`: up to
///     `max_document_type_requirement_slots` (4, `SYSTEM_LIMITS_V5`, backfilled) named
///     slots, each a top-level property the named type must have (`name`: `"$any"`, a
///     property name, or `"$self.<a string property of the referring document>"`;
///     `type`; `maxBytes`; `distinctFrom`, one or several other slots it is filled by a
///     different property than; `when`, a condition on what the named type says of the
///     property), and `require`, `anyOf` / `allOf` over slot names, every slot when absent.
///     `type` may list both kinds, `["permanentDocument", "moderatedDocument"]`: the named
///     type is then either, and the reference is checked as one of the kind the type is (a
///     moderated reference into a moderated type, a permanent one into any other). A
///     slot's `when` is written like a `propertyConstraints` condition (`equal`,
///     `notEqual`, `in`, `notIn`, `contains`, `not`, `anyOf`, `allOf`, `ifThen`,
///     `ifThenElse`) over `$kind` (the kind of the named type), `$property` (the name of the
///     property tried for the slot), `$moderatorAbilities.deleteKeepsFields` and
///     `$moderatorAbilities.changeFields` (the type's lists) and `{ "const": <string> }`,
///     of at most `max_document_type_requirement_condition_nodes` conditions (16,
///     `SYSTEM_LIMITS_V5`, backfilled), each string an `in` or a `notIn` lists counted,
///     none longer than `max_document_type_requirement_condition_text_length` (256,
///     backfilled); a property it does not hold for does not fill the slot. Judging the
///     named type is billed before it is judged, `requirement_credits_per_comparison`
///     (100, added in place to `FEE_DOCUMENT_REFERENCE_VERSION1`, which nothing read for
///     it before) for each property a slot tries and each node of its `when` for it. The native schema check of note 7 admits the keys and meta-schema v3 refuses
///     them. It parses to `DocumentPropertyReferenceTarget::DynamicDocument`,
///     appended. Such a declaration carries nothing else (no `contractId`, `documentType`,
///     `findBy`, `where`, `inList`, `consume` or `constraints`), is no operand of a reference
///     expression, sits on no typed array's elements and in no `ownerRefersTo` or
///     `creatorRefersTo`, and no rule, derived index property or chained query reads through
///     it, each of which resolves the referenced type from the schema. Registration checks
///     the declaration and the properties it reads alone, none of which a moderator may be
///     given to write (`moderatorAbilities.changeFields`). `document_reference_validation` 1
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) checks the target when a document is written:
///     the document names a contract and a document type (`ReferencedEntityNotFoundError`,
///     40120, when it does not), they exist (`ReferencedDocumentTypeNotFoundError`, 40121),
///     the type is not private (`ReferencedDocumentTypePrivateError`, 40150) and not
///     indexOnly (`ReferencedDocumentTypeIndexOnlyError`, 40146), meets the
///     requirements (`ReferencedDocumentTypeRequirementNotMetError`, 40153, new) and is of
///     the declared kind (40122, 40143, 40144), and the document exists (40120) or, for a
///     moderated one a replace keeps, was removed on the record. A replace checks the
///     reference again when its value or a property it reads the target from changed.
///     Protocol v14 and earlier keep `apply_property_reference` 0, which does not read the
///     keys, and `document_reference_validation` 0, which no dynamic reference reaches.
///
/// 34. **A key with a budget or an expiry signs no data contract**: a budget counts what the
///     transitions a key signs take from its identity, and an expiry ends its signing, while a
///     data contract commits its owner beyond the transition that sets it up or changes it.
///     `validate_state_transition_identity_signed` 2 (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`)
///     refuses, unpaid, a `DataContractCreate` and a `DataContractUpdate` signed by a key that
///     carries a budget or an expiry
///     (`PublicKeyWithLimitsCannotSignDataContractTransitionError`, 20018). Protocol v14 keeps
///     `validate_state_transition_identity_signed` 1, so the blocks committed under it replay
///     with the same results.
///
/// 35. **Composite document history**: retained revisions move to per-type history trees,
///     primary entries reference the current revision, and matching queries and verifiers
///     authenticate history pages. The first block crossing into v15 migrates existing
///     history after the operator-payout reconciliation (note 32) and all other activation
///     events in the same transaction. Keep-history index references shrink from 44 to
///     42 bytes by targeting the primary pointer, lowering their reference-size estimates.
///
/// 36. **A contest among documents**: `CONTRACT_VERSIONS_V7` selects
///     `parse_contested_documents_resolution` 0: a contested index may declare
///     `"resolution": 2` (`ContestedIndexResolution::MasternodeVoteOnDocuments`), which the
///     native schema check of note 7 admits and meta-schema v3 refuses. Masternodes vote for
///     a contender, abstain or lock, as under resolution 0, but each document entered for the
///     index values is a contender of its own: the contest keeps it under its document id
///     where the other resolutions keep it under its owner's id, so an identity may enter
///     several (no `DocumentContestIdentityAlreadyContestantError`), a vote towards a
///     contender names the document id, and the stored result names the winning document.
///     The winner is stored under the owner its document names. The methods that write, check
///     and award a contender read the resolution of the contested index, which no contract
///     registered before this version can declare, so the generations protocol v14 and
///     earlier select behave as they did. The decision proposals contract's `choice` type
///     uses it for the choices of a multiple choice proposal. Protocol v14 and earlier keep
///     `parse_contested_documents_resolution` `None` and refuse the value.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: calculate_fee 1 prices a refund at the storing epoch's rate (note 27); apply_contract_with_serialization 1 writes nothing for a contract whose format version 1 bytes match the stored one's (note 8); vote method versions v4: register_identity_vote 1 registers a yes/no vote, remove_all_votes_given_by_identities 1 also removes the yes/no votes of a masternode that left the list, every yes_no method and remove_vote_poll_end_date_query_operations Some(0) make yes/no polls active, add_vote_poll_end_date_query_operations 1 adds only the entry when the batch already queues the end date's tree with other storage flags, and verify methods v4 make the yes/no proof verifiers active (note 10) and verify_private_document_entries Some(0) a page of a private type's store (note 13); state transition method versions v5: document_create_transition 1 deletes a consumed document from what its lookup read (note 11); document method versions v5: insert_empty_private_document_store_operation Some(0) creates a private document type's store, and fetch_private_document_entries and prove_private_document_entries Some(0) read and prove a page of it (note 13); apply_drive_operations 2 resolves a batch's estimate, remove_reference_for_index_level_for_contract_operations 2, add_reference_for_index_level_for_contract_operations 1, stateless_delete_of_non_tree_for_costs 1 and for_total_system_credits_update 1 describe their layers as they are (note 14); create_initial_state_structure 5 adds the token marketplace escrow trees; calculate_total_tokens_balance 2 counts escrowed tokens; the escrow primitives; the identity balance, token balance, prefunded specialized balance and contract fee pot estimations (for_balances 1, for_token_balances 1, estimated_cost_for_prefunded_specialized_balance_update 2, add_estimation_costs_for_contract_fee_pot_update 1) keep a deeper layer description (note 17); fetch_core_fork_activation_height and store_core_fork_activation_height Some(0) keep the Core fork activations (note 18); document method versions v5: append_private_document_entries_operations Some(0) appends a private document create's entries (note 25); grove_version GROVE_V5 (note 26); verify methods v4: verify_proof_keep_serialized 1 verifies a document that may be contested (note 29); document method versions v5: single_document_path_query 1 selects the latest retained revision at or before a block time, and verify_proof_keep_serialized 1 verifies it (note 31); retained-history writers, readers, estimates and verifiers select the per-type revision layout (note 35); add_new_keys_to_masternode_identity 1 adds a masternode key whose hash the identity already holds a reference for (note 32)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: perform_events_on_first_block_of_protocol_change 3 migrates retained history after operator-payout reconciliation and other activation work (note 35); the yes/no poll's tally, record, clean-up and finish hook are Some(0) (note 10); verify_chain_lock_locally 1 rejects a chain lock locally only when Platform holds the quorums active at its signing height, and verify_chain_lock_through_core 1 has Core's verifychainlock check the others before submitchainlock; consensus_params_update 2 installs the full evidence policy (note 16); update_core_info 1 records the chain locked height at which Core's V24 is first seen active (note 18); build_untied_withdrawal_transactions_from_documents 1 emits asset unlock payload version 2 once V24 is recorded as active (note 20); update_voter_identity 1 re-enables the voter key of the address a masternode returns to (note 22); update_masternode_list 2 and update_operator_identity 1 make operator payouts and the operator identities' TRANSFER keys follow Core's diffs (note 32)
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: identity update advanced_structure 1 refuses unpaid an identity update whose added key's proof of possession does not verify (note 2); create_domain_data_trigger 2 (note 3); contract update transform_into_action 1 holds an update to the compiled size cap in check_tx (note 8); batch advanced_structure 2 refuses a token config update on a read-only contract (note 9); masternode vote transform_into_action 1 looks up a masternode's previous answer on a yes/no poll (note 10); document_reference_validation 1 keeps what a consumed commitment's lookup read (note 11); document create structure_validation 2 holds a decision proposal to its fund, rule and end date (note 12); data trigger bindings 3 refuse a replace or a delete of a token market order (note 17) and drop the DashPay bindings (note 24); document create state_validation 3 keeps a create outside a contest off the values of a live one its index values do not decide (note 19); identity create basic_structure 1 checks the identity id before the asset lock proof and advanced_structure 2 no longer does (note 23); contract user moderation transform_into_action 1 writes a moderator's field change in the name of whoever holds the document's storage (note 28); validate_state_transition_identity_signed 2 refuses a contract create or update signed by a key with a budget or an expiry (note 34)
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3,
        query: DRIVE_ABCI_QUERY_VERSIONS_V3, // changed: getDocumentHistory accepts V1 only
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2,
        state_transitions: STATE_TRANSITION_VERSIONS_V5, // changed: validate_asset_lock_transaction_structure 1 drops the input cap and validate_instant_asset_lock_proof_structure 1 applies it to instant proofs only, so a chain proof redeems a lock of more than 100 inputs (note 30)
        contract_versions: CONTRACT_VERSIONS_V7, // changed: parse_property_constraints 1 lets countOf and sumOf match byte array keys (note 4); validate_document 2 checks documents natively (note 6); check_document_type_schema 1 checks schemas natively at registration (note 7); contract_storage_serialization_version 2 stores contracts compiled (note 8); parse_private_document 0 parses private document types (note 13); apply_generated_from 1 admits sys.tokens.decimals (note 15)
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V3, // changed: the yes/no poll's stored info, id, parameter rules and decision rules are active (note 10), and the decision proposal's poll (note 12)
        token_versions: TOKEN_VERSIONS_V3,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V4, // changed: DPNS v3 checks a name's create with keywords (note 3); the decision proposals contract (note 12); the token market contract is selected (note 17); DashPay v3 states what its data triggers checked (note 24)
    fee_version: FEE_VERSION4, // changed: decision proposals prefund their poll with 0.5 Dash (note 12); a private document entry pays its storage and 10,000,000 credits (note 25)
    system_limits: SYSTEM_LIMITS_V5, // changed: max_compiled_contract_size caps a contract stored compiled at 1 MiB (note 8); max_private_document_entry_size caps a private type's entries at 4096 bytes, and the private document store limits size its store (note 13); max_private_document_entries_per_transition caps a private document create at 16 entries (note 25)
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
        evidence: Some(ConsensusEvidenceParams {
            max_bytes: 0,
            max_age_num_blocks: 100_000,
            max_age_duration_seconds: 172_800,
        }),
    },
};
