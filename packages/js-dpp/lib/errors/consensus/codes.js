const ProtocolVersionParsingError = require('./basic/decode/ProtocolVersionParsingError');
const UnsupportedProtocolVersionError = require('./basic/decode/UnsupportedProtocolVersionError');
const IncompatibleProtocolVersionError = require('./basic/decode/IncompatibleProtocolVersionError');
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
const DuplicateDocumentTransitionsError = require('./basic/document/DuplicateDocumentTransitionsError');
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
const IdentityAssetLockProofMismatchError = require('./basic/identity/IdentityAssetLockProofMismatchError');
const IdentityAssetLockTransactionIsNotFoundError = require('./basic/identity/IdentityAssetLockTransactionIsNotFoundError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('./basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const IdentityAssetLockTransactionOutputNotFoundError = require('./basic/identity/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidAssetLockProofCoreChainHeightError = require('./basic/identity/InvalidAssetLockProofCoreChainHeightError');
const InvalidAssetLockProofTransactionHeightError = require('./basic/identity/InvalidAssetLockProofTransactionHeightError');
const InvalidIdentityAssetLockProofError = require('./basic/identity/InvalidIdentityAssetLockProofError');
const InvalidIdentityAssetLockProofSignatureError = require('./basic/identity/InvalidIdentityAssetLockProofSignatureError');
const InvalidIdentityAssetLockTransactionError = require('./basic/identity/InvalidIdentityAssetLockTransactionError');
const InvalidIdentityAssetLockTransactionOutputError = require('./basic/identity/InvalidIdentityAssetLockTransactionOutputError');
const InvalidIdentityPublicKeyDataError = require('./basic/identity/InvalidIdentityPublicKeyDataError');
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
const DuplicateDocumentError = require('./state/document/DuplicateDocumentError');
const InvalidDocumentRevisionError = require('./state/document/InvalidDocumentRevisionError');
const IdentityAlreadyExistsError = require('./state/identity/IdentityAlreadyExistsError');
const IdentityPublicKeyAlreadyExistsError = require('./state/identity/IdentityPublicKeyAlreadyExistsError');

const codes = {
  /**
   * Basic
   */

  // Decoding
  1000: ProtocolVersionParsingError,
  1001: UnsupportedProtocolVersionError,
  1002: IncompatibleProtocolVersionError,
  1003: SerializedObjectParsingError,

  // General
  1004: JsonSchemaError,
  1005: InvalidIdentifierError,

  // Data Contract
  1006: DataContractMaxDepthExceedError,
  1007: DuplicateIndexError,
  1008: InvalidCompoundIndexError,
  1009: InvalidDataContractIdError,
  1010: InvalidIndexedPropertyConstraintError,
  1011: InvalidIndexPropertyTypeError,
  1012: SystemPropertyIndexAlreadyPresentError,
  1013: UndefinedIndexPropertyError,
  1014: UniqueIndicesLimitReachedError,

  // Document
  1015: DataContractNotPresentError,
  1016: DuplicateDocumentTransitionsError,
  1017: InconsistentCompoundIndexDataError,
  1018: InvalidDocumentTransitionActionError,
  1019: InvalidDocumentTransitionIdError,
  1020: InvalidDocumentTypeError,
  1021: MissingDataContractIdError,
  1022: MissingDocumentTransitionActionError,
  1023: MissingDocumentTransitionTypeError,
  1024: MissingDocumentTypeError,

  // Identity
  1025: DuplicatedIdentityPublicKeyError,
  1026: DuplicatedIdentityPublicKeyIdError,
  1027: IdentityAssetLockProofMismatchError,
  1028: IdentityAssetLockTransactionIsNotFoundError,
  1029: IdentityAssetLockTransactionOutPointAlreadyExistsError,
  1030: IdentityAssetLockTransactionOutputNotFoundError,
  1031: InvalidAssetLockProofCoreChainHeightError,
  1032: InvalidAssetLockProofTransactionHeightError,
  1033: InvalidIdentityAssetLockProofError,
  1034: InvalidIdentityAssetLockProofSignatureError,
  1035: InvalidIdentityAssetLockTransactionError,
  1036: InvalidIdentityAssetLockTransactionOutputError,
  1037: InvalidIdentityPublicKeyDataError,

  // State Transition
  1038: InvalidStateTransitionTypeError,
  1039: MissingStateTransitionTypeError,
  1040: StateTransitionMaxSizeExceededError,

  /**
   * Signature
   */

  2000: IdentityNotFoundError,
  2001: InvalidIdentityPublicKeyTypeError,
  2002: InvalidStateTransitionSignatureError,
  2003: MissingPublicKeyError,

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
  4009: DuplicateDocumentError,
  4010: InvalidDocumentRevisionError,

  // Identity
  4011: IdentityAlreadyExistsError,
  4012: IdentityPublicKeyAlreadyExistsError,
};

module.exports = codes;
