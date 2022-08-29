const ProtocolVersionParsingError = require('./basic/decode/ProtocolVersionParsingError');
const UnsupportedProtocolVersionError = require('./basic/UnsupportedProtocolVersionError');
const IncompatibleProtocolVersionError = require('./basic/IncompatibleProtocolVersionError');
const SerializedObjectParsingError = require('./basic/decode/SerializedObjectParsingError');
const JsonSchemaError = require('./basic/JsonSchemaError');
const InvalidIdentifierError = require('./basic/InvalidIdentifierError');
const DataContractMaxDepthExceedError = require('./basic/dataContract/DataContractMaxDepthExceedError');
const DuplicateIndexError = require('./basic/dataContract/DuplicateIndexError');
const InvalidCompoundIndexError = require('./basic/dataContract/InvalidCompoundIndexError');
const InvalidDataContractIdError = require('./basic/dataContract/InvalidDataContractIdError');
const InvalidIndexedPropertyConstraintError = require('./basic/dataContract/InvalidIndexedPropertyConstraintError');
const InvalidIndexPropertyTypeError = require('./basic/dataContract/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('./basic/dataContract/SystemPropertyIndexAlreadyPresentError');
const UndefinedIndexPropertyError = require('./basic/dataContract/UndefinedIndexPropertyError');
const UniqueIndicesLimitReachedError = require('./basic/dataContract/UniqueIndicesLimitReachedError');
const InconsistentCompoundIndexDataError = require('./basic/document/InconsistentCompoundIndexDataError');
const InvalidDocumentTransitionActionError = require('./basic/document/InvalidDocumentTransitionActionError');
const InvalidDocumentTransitionIdError = require('./basic/document/InvalidDocumentTransitionIdError');
const DataContractNotPresentError = require('./basic/document/DataContractNotPresentError');
const InvalidDocumentTypeError = require('./basic/document/InvalidDocumentTypeError');
const MissingDataContractIdError = require('./basic/document/MissingDataContractIdError');
const MissingDocumentTransitionActionError = require('./basic/document/MissingDocumentTransitionActionError');
const MissingDocumentTransitionTypeError = require('./basic/document/MissingDocumentTransitionTypeError');
const MissingDocumentTypeError = require('./basic/document/MissingDocumentTypeError');
const DuplicatedIdentityPublicKeyError = require('./basic/identity/DuplicatedIdentityPublicKeyError');
const DuplicatedIdentityPublicKeyIdError = require('./basic/identity/DuplicatedIdentityPublicKeyIdError');
const MissingMasterPublicKeyError = require('./basic/identity/MissingMasterPublicKeyError');
const IdentityAssetLockProofLockedTransactionMismatchError = require('./basic/identity/IdentityAssetLockProofLockedTransactionMismatchError');
const IdentityAssetLockTransactionIsNotFoundError = require('./basic/identity/IdentityAssetLockTransactionIsNotFoundError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('./basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const IdentityAssetLockTransactionOutputNotFoundError = require('./basic/identity/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidAssetLockProofCoreChainHeightError = require('./basic/identity/InvalidAssetLockProofCoreChainHeightError');
const InvalidAssetLockProofTransactionHeightError = require('./basic/identity/InvalidAssetLockProofTransactionHeightError');
const InvalidIdentityAssetLockTransactionError = require('./basic/identity/InvalidIdentityAssetLockTransactionError');
const InvalidIdentityAssetLockTransactionOutputError = require('./basic/identity/InvalidIdentityAssetLockTransactionOutputError');
const InvalidIdentityPublicKeyDataError = require('./basic/identity/InvalidIdentityPublicKeyDataError');
const InvalidIdentityPublicKeySecurityLevelError = require('./basic/identity/InvalidIdentityPublicKeySecurityLevelError');
const InvalidStateTransitionTypeError = require('./basic/stateTransition/InvalidStateTransitionTypeError');
const MissingStateTransitionTypeError = require('./basic/stateTransition/MissingStateTransitionTypeError');
const StateTransitionMaxSizeExceededError = require('./basic/stateTransition/StateTransitionMaxSizeExceededError');
const IdentityNotFoundError = require('./signature/IdentityNotFoundError');
const InvalidIdentityPublicKeyTypeError = require('./signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('./signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyError = require('./signature/MissingPublicKeyError');
const BalanceIsNotEnoughError = require('./fee/BalanceIsNotEnoughError');
const DataContractAlreadyPresentError = require('./state/dataContract/DataContractAlreadyPresentError');
const DataTriggerConditionError = require('./state/dataContract/dataTrigger/DataTriggerConditionError');
const DataTriggerExecutionError = require('./state/dataContract/dataTrigger/DataTriggerExecutionError');
const DataTriggerInvalidResultError = require('./state/dataContract/dataTrigger/DataTriggerInvalidResultError');
const DocumentAlreadyPresentError = require('./state/document/DocumentAlreadyPresentError');
const DocumentNotFoundError = require('./state/document/DocumentNotFoundError');
const DocumentOwnerIdMismatchError = require('./state/document/DocumentOwnerIdMismatchError');
const DocumentTimestampsMismatchError = require('./state/document/DocumentTimestampsMismatchError');
const DocumentTimestampWindowViolationError = require('./state/document/DocumentTimestampWindowViolationError');
const DuplicateUniqueIndexError = require('./state/document/DuplicateUniqueIndexError');
const InvalidDocumentRevisionError = require('./state/document/InvalidDocumentRevisionError');
const IdentityAlreadyExistsError = require('./state/identity/IdentityAlreadyExistsError');
const InvalidJsonSchemaRefError = require('./basic/dataContract/InvalidJsonSchemaRefError');
const JsonSchemaCompilationError = require('./basic/JsonSchemaCompilationError');
const DuplicateDocumentTransitionsWithIdsError = require('./basic/document/DuplicateDocumentTransitionsWithIdsError');
const DuplicateDocumentTransitionsWithIndicesError = require('./basic/document/DuplicateDocumentTransitionsWithIndicesError');
const InvalidAssetLockTransactionOutputReturnSizeError = require('./basic/identity/InvalidAssetLockTransactionOutputReturnSizeError');
const InvalidInstantAssetLockProofError = require('./basic/identity/InvalidInstantAssetLockProofError');
const InvalidInstantAssetLockProofSignatureError = require('./basic/identity/InvalidInstantAssetLockProofSignatureError');
const IncompatibleRe2PatternError = require('./basic/dataContract/IncompatibleRe2PatternError');
const InvalidDataContractVersionError = require('./basic/dataContract/InvalidDataContractVersionError');
const IncompatibleDataContractSchemaError = require('./basic/dataContract/IncompatibleDataContractSchemaError');
const DataContractImmutablePropertiesUpdateError = require('./basic/dataContract/DataContractImmutablePropertiesUpdateError');
const DataContractIndicesChangedError = require('./basic/dataContract/DataContractUniqueIndicesChangedError');
const DuplicateIndexNameError = require('./basic/dataContract/DuplicateIndexNameError');
const DataContractInvalidIndexDefinitionUpdateError = require('./basic/dataContract/DataContractInvalidIndexDefinitionUpdateError');
const DataContractHaveNewUniqueIndexError = require('./basic/dataContract/DataContractHaveNewUniqueIndexError');
const IdentityPublicKeyDisabledAtWindowViolationError = require('./state/identity/IdentityPublicKeyDisabledAtWindowViolationError');
const IdentityPublicKeyIsReadOnlyError = require('./state/identity/IdentityPublicKeyIsReadOnlyError');
const InvalidIdentityPublicKeyIdError = require('./state/identity/InvalidIdentityPublicKeyIdError');
const InvalidIdentityRevisionError = require('./state/identity/InvalidIdentityRevisionError');
const StateMaxIdentityPublicKeyLimitReachedError = require('./state/identity/MaxIdentityPublicKeyLimitReachedError');
const DuplicatedIdentityPublicKeyStateError = require('./state/identity/DuplicatedIdentityPublicKeyError');
const DuplicatedIdentityPublicKeyIdStateError = require('./state/identity/DuplicatedIdentityPublicKeyIdError');
const InvalidIdentityKeySignatureError = require('./basic/identity/InvalidIdentityKeySignatureError');
const InvalidSignaturePublicKeySecurityLevelError = require('./signature/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeyIsDisabledError = require('./signature/PublicKeyIsDisabledError');
const PublicKeySecurityLevelNotMetError = require('./signature/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('./signature/WrongPublicKeyPurposeError');
const IdentityPublicKeyIsDisabledError = require('./state/identity/IdentityPublicKeyIsDisabledError');

const codes = {
  /**
   * Basic
   */

  // Decoding
  1000: ProtocolVersionParsingError,
  1001: SerializedObjectParsingError,

  // General
  1002: UnsupportedProtocolVersionError,
  1003: IncompatibleProtocolVersionError,
  1004: JsonSchemaCompilationError,
  1005: JsonSchemaError,
  1006: InvalidIdentifierError,

  // Data Contract
  1007: DataContractMaxDepthExceedError,
  1008: DuplicateIndexError,
  1009: IncompatibleRe2PatternError,
  1010: InvalidCompoundIndexError,
  1011: InvalidDataContractIdError,
  1012: InvalidIndexedPropertyConstraintError,
  1013: InvalidIndexPropertyTypeError,
  1014: InvalidJsonSchemaRefError,
  1015: SystemPropertyIndexAlreadyPresentError,
  1016: UndefinedIndexPropertyError,
  1017: UniqueIndicesLimitReachedError,
  1048: DuplicateIndexNameError,
  1050: InvalidDataContractVersionError,
  1051: IncompatibleDataContractSchemaError,
  1052: DataContractImmutablePropertiesUpdateError,
  1053: DataContractIndicesChangedError,
  1054: DataContractInvalidIndexDefinitionUpdateError,
  1055: DataContractHaveNewUniqueIndexError,

  // Document
  1018: DataContractNotPresentError,
  1019: DuplicateDocumentTransitionsWithIdsError,
  1020: DuplicateDocumentTransitionsWithIndicesError,
  1021: InconsistentCompoundIndexDataError,
  1022: InvalidDocumentTransitionActionError,
  1023: InvalidDocumentTransitionIdError,
  1024: InvalidDocumentTypeError,
  1025: MissingDataContractIdError,
  1026: MissingDocumentTransitionActionError,
  1027: MissingDocumentTransitionTypeError,
  1028: MissingDocumentTypeError,

  // Identity
  1029: DuplicatedIdentityPublicKeyError,
  1030: DuplicatedIdentityPublicKeyIdError,
  1031: IdentityAssetLockProofLockedTransactionMismatchError,
  1032: IdentityAssetLockTransactionIsNotFoundError,
  1033: IdentityAssetLockTransactionOutPointAlreadyExistsError,
  1034: IdentityAssetLockTransactionOutputNotFoundError,
  1035: InvalidAssetLockProofCoreChainHeightError,
  1036: InvalidAssetLockProofTransactionHeightError,
  1037: InvalidAssetLockTransactionOutputReturnSizeError,
  1038: InvalidIdentityAssetLockTransactionError,
  1039: InvalidIdentityAssetLockTransactionOutputError,
  1040: InvalidIdentityPublicKeyDataError,
  1041: InvalidInstantAssetLockProofError,
  1042: InvalidInstantAssetLockProofSignatureError,
  1046: MissingMasterPublicKeyError,
  1047: InvalidIdentityPublicKeySecurityLevelError,
  1056: InvalidIdentityKeySignatureError,

  // State Transition
  1043: InvalidStateTransitionTypeError,
  1044: MissingStateTransitionTypeError,
  1045: StateTransitionMaxSizeExceededError,

  /**
   * Signature
   */

  2000: IdentityNotFoundError,
  2001: InvalidIdentityPublicKeyTypeError,
  2002: InvalidStateTransitionSignatureError,
  2003: MissingPublicKeyError,
  2004: InvalidSignaturePublicKeySecurityLevelError,
  2005: WrongPublicKeyPurposeError,
  2006: PublicKeyIsDisabledError,
  2007: PublicKeySecurityLevelNotMetError,

  /**
   * Fee
   */

  3000: BalanceIsNotEnoughError,

  /**
   * State
   */

  // Data Contract
  4000: DataContractAlreadyPresentError,
  4001: DataTriggerConditionError,
  4002: DataTriggerExecutionError,
  4003: DataTriggerInvalidResultError,

  // Document
  4004: DocumentAlreadyPresentError,
  4005: DocumentNotFoundError,
  4006: DocumentOwnerIdMismatchError,
  4007: DocumentTimestampsMismatchError,
  4008: DocumentTimestampWindowViolationError,
  4009: DuplicateUniqueIndexError,
  4010: InvalidDocumentRevisionError,

  // Identity
  4011: IdentityAlreadyExistsError,
  4012: IdentityPublicKeyDisabledAtWindowViolationError,
  4017: IdentityPublicKeyIsReadOnlyError,
  4018: InvalidIdentityPublicKeyIdError,
  4019: InvalidIdentityRevisionError,
  4020: StateMaxIdentityPublicKeyLimitReachedError,
  4021: DuplicatedIdentityPublicKeyStateError,
  4022: DuplicatedIdentityPublicKeyIdStateError,
  4023: IdentityPublicKeyIsDisabledError,
};

module.exports = codes;
