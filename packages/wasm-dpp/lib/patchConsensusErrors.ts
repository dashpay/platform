import * as dpp_module from '../wasm/wasm_dpp';
import { extend } from "./extend";
import { AbstractConsensusError } from "./AbstractConsensusError";

const {
    ProtocolVersionParsingError,
    UnsupportedProtocolVersionError,
    IncompatibleProtocolVersionError,
    SerializedObjectParsingError,
    JsonSchemaError,
    InvalidIdentifierError,
    DataContractMaxDepthExceedError,
    DuplicateIndexError,
    InvalidCompoundIndexError,
    InvalidDataContractIdError,
    InvalidIndexedPropertyConstraintError,
    InvalidIndexPropertyTypeError,
    SystemPropertyIndexAlreadyPresentError,
    UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError,
    InconsistentCompoundIndexDataError,
    InvalidDocumentTransitionActionError,
    InvalidDocumentTransitionIdError,
    DataContractNotPresentError,
    InvalidDocumentTypeError,
    MissingDataContractIdError,
    MissingDocumentTransitionActionError,
    MissingDocumentTransitionTypeError,
    MissingDocumentTypeError,
    DuplicatedIdentityPublicKeyError,
    DuplicatedIdentityPublicKeyIdError,
    MissingMasterPublicKeyError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError,
    InvalidAssetLockProofCoreChainHeightError,
    InvalidAssetLockProofTransactionHeightError,
    InvalidIdentityAssetLockTransactionError,
    InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityPublicKeyDataError,
    InvalidIdentityPublicKeySecurityLevelError,
    InvalidStateTransitionTypeError,
    MissingStateTransitionTypeError,
    StateTransitionMaxSizeExceededError,
    IdentityNotFoundError,
    InvalidIdentityPublicKeyTypeError,
    InvalidStateTransitionSignatureError,
    MissingPublicKeyError,
    BalanceIsNotEnoughError,
    DataContractAlreadyPresentError,
    DataTriggerConditionError,
    DataTriggerExecutionError,
    DataTriggerInvalidResultError,
    DocumentAlreadyPresentError,
    DocumentNotFoundError,
    DocumentOwnerIdMismatchError,
    DocumentTimestampsMismatchError,
    DocumentTimestampWindowViolationError,
    DuplicateUniqueIndexError,
    InvalidDocumentRevisionError,
    IdentityAlreadyExistsError,
    InvalidJsonSchemaRefError,
    JsonSchemaCompilationError,
    DuplicateDocumentTransitionsWithIdsError,
    DuplicateDocumentTransitionsWithIndicesError,
    InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidInstantAssetLockProofError,
    InvalidInstantAssetLockProofSignatureError,
    IncompatibleRe2PatternError,
    InvalidDataContractVersionError,
    IncompatibleDataContractSchemaError,
    DataContractImmutablePropertiesUpdateError,
    DataContractIndicesChangedError,
    DuplicateIndexNameError,
    DataContractInvalidIndexDefinitionUpdateError,
    DataContractHaveNewUniqueIndexError,
    IdentityPublicKeyDisabledAtWindowViolationError,
    IdentityPublicKeyIsReadOnlyError,
    InvalidIdentityPublicKeyIdError,
    InvalidIdentityRevisionError,
    StateMaxIdentityPublicKeyLimitReachedError,
    DuplicatedIdentityPublicKeyStateError,
    DuplicatedIdentityPublicKeyIdStateError,
    InvalidIdentityKeySignatureError,
    InvalidSignaturePublicKeySecurityLevelError,
    PublicKeyIsDisabledError,
    PublicKeySecurityLevelNotMetError,
    WrongPublicKeyPurposeError,
    IdentityPublicKeyIsDisabledError,
} = dpp_module;

export function patchConsensusErrors() {
    extend(ProtocolVersionParsingError, AbstractConsensusError);
    extend(UnsupportedProtocolVersionError, AbstractConsensusError);
    extend(IncompatibleProtocolVersionError, AbstractConsensusError);
    extend(SerializedObjectParsingError, AbstractConsensusError);
    extend(JsonSchemaError, AbstractConsensusError);
    extend(InvalidIdentifierError, AbstractConsensusError);
    extend(DataContractMaxDepthExceedError, AbstractConsensusError);
    extend(DuplicateIndexError, AbstractConsensusError);
    extend(InvalidCompoundIndexError, AbstractConsensusError);
    extend(InvalidDataContractIdError, AbstractConsensusError);
    extend(InvalidIndexedPropertyConstraintError, AbstractConsensusError);
    extend(InvalidIndexPropertyTypeError, AbstractConsensusError);
    extend(SystemPropertyIndexAlreadyPresentError, AbstractConsensusError);
    extend(UndefinedIndexPropertyError, AbstractConsensusError);
    extend(UniqueIndicesLimitReachedError, AbstractConsensusError);
    extend(InconsistentCompoundIndexDataError, AbstractConsensusError);
    extend(InvalidDocumentTransitionActionError, AbstractConsensusError);
    extend(InvalidDocumentTransitionIdError, AbstractConsensusError);
    extend(DataContractNotPresentError, AbstractConsensusError);
    extend(InvalidDocumentTypeError, AbstractConsensusError);
    extend(MissingDataContractIdError, AbstractConsensusError);
    extend(MissingDocumentTransitionActionError, AbstractConsensusError);
    extend(MissingDocumentTransitionTypeError, AbstractConsensusError);
    extend(MissingDocumentTypeError, AbstractConsensusError);
    extend(DuplicatedIdentityPublicKeyError, AbstractConsensusError);
    extend(DuplicatedIdentityPublicKeyIdError, AbstractConsensusError);
    extend(MissingMasterPublicKeyError, AbstractConsensusError);
    extend(IdentityAssetLockProofLockedTransactionMismatchError, AbstractConsensusError);
    extend(IdentityAssetLockTransactionIsNotFoundError, AbstractConsensusError);
    extend(IdentityAssetLockTransactionOutPointAlreadyExistsError, AbstractConsensusError);
    extend(IdentityAssetLockTransactionOutputNotFoundError, AbstractConsensusError);
    extend(InvalidAssetLockProofCoreChainHeightError, AbstractConsensusError);
    extend(InvalidAssetLockProofTransactionHeightError, AbstractConsensusError);
    extend(InvalidIdentityAssetLockTransactionError, AbstractConsensusError);
    extend(InvalidIdentityAssetLockTransactionOutputError, AbstractConsensusError);
    extend(InvalidIdentityPublicKeyDataError, AbstractConsensusError);
    extend(InvalidIdentityPublicKeySecurityLevelError, AbstractConsensusError);
    extend(InvalidStateTransitionTypeError, AbstractConsensusError);
    extend(MissingStateTransitionTypeError, AbstractConsensusError);
    extend(StateTransitionMaxSizeExceededError, AbstractConsensusError);
    extend(IdentityNotFoundError, AbstractConsensusError);
    extend(InvalidIdentityPublicKeyTypeError, AbstractConsensusError);
    extend(InvalidStateTransitionSignatureError, AbstractConsensusError);
    extend(MissingPublicKeyError, AbstractConsensusError);
    extend(BalanceIsNotEnoughError, AbstractConsensusError);
    extend(DataContractAlreadyPresentError, AbstractConsensusError);
    extend(DataTriggerConditionError, AbstractConsensusError);
    extend(DataTriggerExecutionError, AbstractConsensusError);
    extend(DataTriggerInvalidResultError, AbstractConsensusError);
    extend(DocumentAlreadyPresentError, AbstractConsensusError);
    extend(DocumentNotFoundError, AbstractConsensusError);
    extend(DocumentOwnerIdMismatchError, AbstractConsensusError);
    extend(DocumentTimestampsMismatchError, AbstractConsensusError);
    extend(DocumentTimestampWindowViolationError, AbstractConsensusError);
    extend(DuplicateUniqueIndexError, AbstractConsensusError);
    extend(InvalidDocumentRevisionError, AbstractConsensusError);
    extend(IdentityAlreadyExistsError, AbstractConsensusError);
    extend(InvalidJsonSchemaRefError, AbstractConsensusError);
    extend(JsonSchemaCompilationError, AbstractConsensusError);
    extend(DuplicateDocumentTransitionsWithIdsError, AbstractConsensusError);
    extend(DuplicateDocumentTransitionsWithIndicesError, AbstractConsensusError);
    extend(InvalidAssetLockTransactionOutputReturnSizeError, AbstractConsensusError);
    extend(InvalidInstantAssetLockProofError, AbstractConsensusError);
    extend(InvalidInstantAssetLockProofSignatureError, AbstractConsensusError);
    extend(IncompatibleRe2PatternError, AbstractConsensusError);
    extend(InvalidDataContractVersionError, AbstractConsensusError);
    extend(IncompatibleDataContractSchemaError, AbstractConsensusError);
    extend(DataContractImmutablePropertiesUpdateError, AbstractConsensusError);
    extend(DataContractIndicesChangedError, AbstractConsensusError);
    extend(DuplicateIndexNameError, AbstractConsensusError);
    extend(DataContractInvalidIndexDefinitionUpdateError, AbstractConsensusError);
    extend(DataContractHaveNewUniqueIndexError, AbstractConsensusError);
    extend(IdentityPublicKeyDisabledAtWindowViolationError, AbstractConsensusError);
    extend(IdentityPublicKeyIsReadOnlyError, AbstractConsensusError);
    extend(InvalidIdentityPublicKeyIdError, AbstractConsensusError);
    extend(InvalidIdentityRevisionError, AbstractConsensusError);
    extend(StateMaxIdentityPublicKeyLimitReachedError, AbstractConsensusError);
    extend(DuplicatedIdentityPublicKeyStateError, AbstractConsensusError);
    extend(DuplicatedIdentityPublicKeyIdStateError, AbstractConsensusError);
    extend(InvalidIdentityKeySignatureError, AbstractConsensusError);
    extend(InvalidSignaturePublicKeySecurityLevelError, AbstractConsensusError);
    extend(PublicKeyIsDisabledError, AbstractConsensusError);
    extend(PublicKeySecurityLevelNotMetError, AbstractConsensusError);
    extend(WrongPublicKeyPurposeError, AbstractConsensusError);
    extend(IdentityPublicKeyIsDisabledError, AbstractConsensusError);
}