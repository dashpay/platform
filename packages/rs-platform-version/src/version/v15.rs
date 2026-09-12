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
use crate::version::dpp_versions::dpp_voting_versions::v3::VOTING_VERSION_V3;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
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
///    the parent's subdomain rule. The data trigger bindings stay at 2: their
///    `domain` Replace and Delete rejects are never reached, as
///    `documentsMutable` and `canBeDeleted` false refuse both first (10404).
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
///     (`DRIVE_ABCI_VALIDATION_VERSIONS_V11`) reads the commitment that the `refersTo` lookup of
///     a reference declaring `consume` finds with its stored element's storage flags
///     (`query_documents_with_flags`), and keeps that read in the `ConsumedDocument`. Any other
///     lookup reads as document reference validation 0's does. The create
///     conversion 1 (`DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5`, which `DRIVE_VERSION_V10`
///     selects) deletes the document from it
///     (`Drive::force_delete_read_document_for_contract_operations`, the force delete's
///     operations less its read), where conversion 0 deletes it by id and bills a second read.
///     A consume costs one document read less: a DPNS v3 name create's processing fee drops by
///     10,660 credits. Storage fees and refunds are unchanged. Protocol v14 and earlier keep
///     document reference validation 0 and the create conversion 0.
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
///     change it (`DocumentTypeUpdateError`). No reference may name a private type: a
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
///
/// 14. **Fee history required for storage refunds**: `DRIVE_VERSION_V10`
///     bumps `fees.calculate_fee` to 1. A storage refund for owner-attributed
///     bytes is priced only with the fee history of the block that removes
///     the bytes; the history is consulted on every fee version number and a
///     missing history is an internal error instead of a silent fallback to
///     the first-generation storage rates. Every shipped schedule shares fee
///     version number 1 and the same storage rates, so refund credits are
///     unchanged for every shipped input; the boundary makes the absence of
///     history an error from this version onward.
///
/// 15. **Recorded-owner refund credits**: the same drive table turns on
///     `identity.update.credit_storage_refunds_to_owners`, the primitive that
///     credits each recorded owner of a refund without consulting any key or
///     permission and reports the amount whose owner has no balance element,
///     so block lifecycle paths can settle it into the current epoch's
///     processing pool.
///
/// Everything else matches v14.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: apply_contract_with_serialization 1 writes nothing for a contract whose format version 1 bytes match the stored one's (note 8); calculate_fee v1 (fee history required for refunds) + identity table v3 (credit_storage_refunds_to_owners); vote method versions v4: register_identity_vote 1 registers a yes/no vote, remove_all_votes_given_by_identities 1 also removes the yes/no votes of a masternode that left the list, every yes_no method and remove_vote_poll_end_date_query_operations Some(0) make yes/no polls active, add_vote_poll_end_date_query_operations 1 adds only the entry when the batch already queues the end date's tree with other storage flags, and verify methods v4 make the yes/no proof verifiers active (note 10); state transition method versions v5: document_create_transition 1 deletes a consumed document from what its lookup read (note 11)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: the yes/no poll's tally, record, clean-up and finish hook are Some(0) (note 10); verify_chain_lock_locally 1 rejects a chain lock locally only when Platform holds the quorums active at its signing height, and verify_chain_lock_through_core 1 has Core's verifychainlock check the others before submitchainlock
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: identity update advanced_structure 1 refuses unpaid an identity update whose added key's proof of possession does not verify (note 2); create_domain_data_trigger 2 (note 3); contract update transform_into_action 1 holds an update to the compiled size cap in check_tx (note 8); batch advanced_structure 2 refuses a token config update on a read-only contract (note 9); masternode vote transform_into_action 1 looks up a masternode's previous answer on a yes/no poll (note 10); document_reference_validation 1 keeps what a consumed commitment's lookup read (note 11); document create structure_validation 2 holds a decision proposal to its fund, rule and end date (note 12)
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
        contract_versions: CONTRACT_VERSIONS_V7, // changed: parse_property_constraints 1 lets countOf and sumOf match byte array keys (note 4); validate_document 2 checks documents natively (note 6); check_document_type_schema 1 checks schemas natively at registration (note 7); contract_storage_serialization_version 2 stores contracts compiled (note 8); parse_private_document 0 parses private document types (note 13)
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V3, // changed: the yes/no poll's stored info, id, parameter rules and decision rules are active (note 10), and the decision proposal's poll (note 12)
        token_versions: TOKEN_VERSIONS_V3,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V4, // changed: DPNS v3 checks a name's create with keywords (note 3); the decision proposals contract (note 12)
    fee_version: FEE_VERSION4, // changed: decision proposals prefund their poll with 0.5 Dash (note 12)
    system_limits: SYSTEM_LIMITS_V5, // changed: max_compiled_contract_size caps a contract stored compiled at 1 MiB (note 8); max_private_document_entry_size caps a private type's entries at 4096 bytes (note 13)
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
