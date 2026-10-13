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
use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v2::DRIVE_ABCI_STRUCTURE_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v11::DRIVE_ABCI_VALIDATION_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
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
/// Changes from v14:
///
/// 1. **DPNS v3 checks a name's create with keywords**:
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
///    when consumed). The `domain` keeps `preorderSalt` declared, transient and
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
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V9,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: create_domain_data_trigger 2 (note 1)
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
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V4, // changed: DPNS v3 checks a name's create with keywords (note 1)
    fee_version: FEE_VERSION3,
    system_limits: SYSTEM_LIMITS_V4,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
