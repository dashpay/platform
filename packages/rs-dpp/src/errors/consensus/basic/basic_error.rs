use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::{
    ContestedUniqueIndexOnMutableDocumentTypeError, ContestedUniqueIndexWithUniqueIndexError,
    DataContractHaveNewUniqueIndexError, DataContractImmutablePropertiesUpdateError,
    DataContractInvalidIndexDefinitionUpdateError, DataContractTokenConfigurationUpdateError,
    DataContractUniqueIndicesChangedError, DecimalsOverLimitError, DuplicateIndexError,
    DuplicateIndexNameError, GroupExceedsMaxMembersError, GroupHasTooFewMembersError,
    GroupMemberHasPowerOfZeroError, GroupMemberHasPowerOverLimitError,
    GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError, GroupPositionDoesNotExistError,
    GroupRequiredPowerIsInvalidError, GroupTotalPowerLessThanRequiredError,
    IncompatibleDataContractSchemaError, IncompatibleDocumentTypeSchemaError,
    IncompatibleRe2PatternError, InvalidCompoundIndexError, InvalidDataContractIdError,
    InvalidDataContractVersionError, InvalidDocumentTypeNameError,
    InvalidDocumentTypeRequiredSecurityLevelError, InvalidIndexPropertyTypeError,
    InvalidIndexedPropertyConstraintError, InvalidKeywordCharacterError,
    InvalidTokenBaseSupplyError, InvalidTokenDistributionFunctionDivideByZeroError,
    InvalidTokenDistributionFunctionIncoherenceError,
    InvalidTokenDistributionFunctionInvalidParameterError,
    InvalidTokenDistributionFunctionInvalidParameterTupleError, InvalidTokenLanguageCodeError,
    InvalidTokenNameCharacterError, InvalidTokenNameLengthError, MainGroupIsNotDefinedError,
    NewTokensDestinationIdentityOptionRequiredError, NonContiguousContractGroupPositionsError,
    NonContiguousContractTokenPositionsError, RedundantDocumentPaidForByTokenWithContractId,
    SystemPropertyIndexAlreadyPresentError, UndefinedIndexPropertyError,
    UniqueIndicesLimitReachedError, UnknownDocumentCreationRestrictionModeError,
    UnknownGasFeesPaidByError, UnknownSecurityLevelError, UnknownStorageKeyRequirementsError,
    UnknownTradeModeError, UnknownTransferableTypeError,
};
use crate::consensus::basic::data_contract::{
    InvalidJsonSchemaRefError, TokenPaymentByBurningOnlyAllowedOnInternalTokenError,
    UnknownDocumentActionTokenEffectError,
};
use crate::consensus::basic::decode::{
    ProtocolVersionParsingError, SerializedObjectParsingError, VersionError,
};
use crate::consensus::basic::document::{
    ContestedDocumentsTemporarilyNotAllowedError, DataContractNotPresentError,
    DocumentCreationNotAllowedError, DocumentFieldMaxSizeExceededError,
    DocumentTransitionsAreAbsentError, DuplicateDocumentTransitionsWithIdsError,
    DuplicateDocumentTransitionsWithIndicesError, InconsistentCompoundIndexDataError,
    InvalidDocumentTransitionActionError, InvalidDocumentTransitionIdError,
    InvalidDocumentTypeError, MaxDocumentsTransitionsExceededError,
    MissingDataContractIdBasicError, MissingDocumentTransitionActionError,
    MissingDocumentTransitionTypeError, MissingDocumentTypeError,
    MissingPositionsInDocumentTypePropertiesError, NonceOutOfBoundsError,
};
use crate::consensus::basic::identity::{
    DataContractBoundsNotPresentError, DisablingKeyIdAlsoBeingAddedInSameTransitionError,
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockStateTransitionReplayError, IdentityAssetLockTransactionIsNotFoundError,
    IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
    IdentityAssetLockTransactionOutputNotFoundError, IdentityCreditTransferToSelfError,
    InvalidAssetLockProofCoreChainHeightError, InvalidAssetLockProofTransactionHeightError,
    InvalidAssetLockTransactionOutputReturnSizeError,
    InvalidIdentityAssetLockProofChainLockValidationError,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
    InvalidIdentityCreditTransferAmountError, InvalidIdentityCreditWithdrawalTransitionAmountError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError, InvalidIdentityKeySignatureError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    InvalidIdentityUpdateTransitionDisableKeysError, InvalidIdentityUpdateTransitionEmptyError,
    InvalidInstantAssetLockProofError, InvalidInstantAssetLockProofSignatureError,
    InvalidKeyPurposeForContractBoundsError, MissingMasterPublicKeyError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError, TooManyMasterPublicKeyError,
    WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError,
};
use crate::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use crate::consensus::basic::state_transition::{
    InvalidStateTransitionTypeError, MissingStateTransitionTypeError,
    StateTransitionMaxSizeExceededError,
};
use crate::consensus::basic::{
    IncompatibleProtocolVersionError, UnsupportedFeatureError, UnsupportedProtocolVersionError,
};
use crate::consensus::ConsensusError;

use super::data_contract::{
    DuplicateKeywordsError, InvalidDescriptionLengthError, InvalidKeywordLengthError,
    TooManyKeywordsError,
};
use crate::consensus::basic::group::GroupActionNotAllowedOnTransitionError;
use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::token::{
    ChoosingTokenMintRecipientNotAllowedError, ContractHasNoTokensError,
    DestinationIdentityForTokenMintingNotSetError, InvalidActionIdError, InvalidTokenAmountError,
    InvalidTokenConfigUpdateNoChangeError, InvalidTokenDistributionBlockIntervalTooShortError,
    InvalidTokenDistributionTimeIntervalNotMinuteAlignedError,
    InvalidTokenDistributionTimeIntervalTooShortError, InvalidTokenIdError,
    InvalidTokenNoteTooBigError, InvalidTokenPositionError, MissingDefaultLocalizationError,
    TokenNoteOnlyAllowedWhenProposerError, TokenTransferToOurselfError,
};
use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::{
    json_schema_compilation_error::JsonSchemaCompilationError, json_schema_error::JsonSchemaError,
};
use crate::consensus::state::identity::master_public_key_update_error::MasterPublicKeyUpdateError;
use crate::data_contract::errors::DataContractError;

#[allow(clippy::large_enum_variant)]
#[derive(
    Error, Debug, PlatformSerialize, PlatformDeserialize, Encode, Decode, PartialEq, Clone,
)]
pub enum BasicError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    // Decoding
    #[error(transparent)]
    ProtocolVersionParsingError(ProtocolVersionParsingError),

    #[error(transparent)]
    VersionError(VersionError),

    #[error(transparent)]
    ContractError(DataContractError),

    #[error(transparent)]
    UnknownSecurityLevelError(UnknownSecurityLevelError),

    #[error(transparent)]
    UnknownStorageKeyRequirementsError(UnknownStorageKeyRequirementsError),

    #[error(transparent)]
    UnknownTransferableTypeError(UnknownTransferableTypeError),

    #[error(transparent)]
    UnknownTradeModeError(UnknownTradeModeError),

    #[error(transparent)]
    UnknownDocumentCreationRestrictionModeError(UnknownDocumentCreationRestrictionModeError),

    #[error(transparent)]
    SerializedObjectParsingError(SerializedObjectParsingError),

    #[error(transparent)]
    UnsupportedProtocolVersionError(UnsupportedProtocolVersionError),

    #[error(transparent)]
    UnsupportedVersionError(UnsupportedVersionError),

    #[error(transparent)]
    IncompatibleProtocolVersionError(IncompatibleProtocolVersionError),

    // Structure error
    #[error(transparent)]
    JsonSchemaCompilationError(JsonSchemaCompilationError),

    #[error(transparent)]
    JsonSchemaError(JsonSchemaError),

    #[error(transparent)]
    InvalidIdentifierError(InvalidIdentifierError),

    #[error(transparent)]
    ValueError(ValueError),

    // DataContract
    #[error(transparent)]
    DataContractMaxDepthExceedError(DataContractMaxDepthExceedError),

    #[error(transparent)]
    DuplicateIndexError(DuplicateIndexError),

    #[error(transparent)]
    IncompatibleRe2PatternError(IncompatibleRe2PatternError),

    #[error(transparent)]
    InvalidCompoundIndexError(InvalidCompoundIndexError),

    #[error(transparent)]
    InvalidDataContractIdError(InvalidDataContractIdError),

    #[error(transparent)]
    InvalidIndexedPropertyConstraintError(InvalidIndexedPropertyConstraintError),

    #[error(transparent)]
    InvalidIndexPropertyTypeError(InvalidIndexPropertyTypeError),

    #[error(transparent)]
    InvalidJsonSchemaRefError(InvalidJsonSchemaRefError),

    #[error(transparent)]
    SystemPropertyIndexAlreadyPresentError(SystemPropertyIndexAlreadyPresentError),

    #[error(transparent)]
    UndefinedIndexPropertyError(UndefinedIndexPropertyError),

    #[error(transparent)]
    UniqueIndicesLimitReachedError(UniqueIndicesLimitReachedError),

    #[error(transparent)]
    DuplicateIndexNameError(DuplicateIndexNameError),

    #[error(transparent)]
    InvalidDataContractVersionError(InvalidDataContractVersionError),

    #[error(transparent)]
    IncompatibleDataContractSchemaError(IncompatibleDataContractSchemaError),

    #[error(transparent)]
    DataContractImmutablePropertiesUpdateError(DataContractImmutablePropertiesUpdateError),

    #[error(transparent)]
    DataContractUniqueIndicesChangedError(DataContractUniqueIndicesChangedError),

    #[error(transparent)]
    DataContractInvalidIndexDefinitionUpdateError(DataContractInvalidIndexDefinitionUpdateError),

    #[error(transparent)]
    DataContractHaveNewUniqueIndexError(DataContractHaveNewUniqueIndexError),

    // Document
    #[error(transparent)]
    DataContractNotPresentError(DataContractNotPresentError),

    #[error(transparent)]
    DocumentCreationNotAllowedError(DocumentCreationNotAllowedError),

    #[error(transparent)]
    DataContractBoundsNotPresentError(DataContractBoundsNotPresentError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIdsError(DuplicateDocumentTransitionsWithIdsError),

    #[error(transparent)]
    DuplicateDocumentTransitionsWithIndicesError(DuplicateDocumentTransitionsWithIndicesError),

    #[error(transparent)]
    NonceOutOfBoundsError(NonceOutOfBoundsError),

    #[error(transparent)]
    InconsistentCompoundIndexDataError(InconsistentCompoundIndexDataError),

    #[error(transparent)]
    InvalidDocumentTransitionActionError(InvalidDocumentTransitionActionError),

    #[error(transparent)]
    InvalidDocumentTransitionIdError(InvalidDocumentTransitionIdError),

    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error(transparent)]
    MissingPositionsInDocumentTypePropertiesError(MissingPositionsInDocumentTypePropertiesError),

    #[error(transparent)]
    MissingDataContractIdBasicError(MissingDataContractIdBasicError),

    #[error(transparent)]
    MissingDocumentTransitionActionError(MissingDocumentTransitionActionError),

    #[error(transparent)]
    MissingDocumentTransitionTypeError(MissingDocumentTransitionTypeError),

    #[error(transparent)]
    MissingDocumentTypeError(MissingDocumentTypeError),

    #[error(transparent)]
    MaxDocumentsTransitionsExceededError(MaxDocumentsTransitionsExceededError),

    // Identity
    #[error(transparent)]
    DuplicatedIdentityPublicKeyBasicError(DuplicatedIdentityPublicKeyBasicError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyIdBasicError(DuplicatedIdentityPublicKeyIdBasicError),

    #[error(transparent)]
    DisablingKeyIdAlsoBeingAddedInSameTransitionError(
        DisablingKeyIdAlsoBeingAddedInSameTransitionError,
    ),

    #[error(transparent)]
    IdentityAssetLockProofLockedTransactionMismatchError(
        IdentityAssetLockProofLockedTransactionMismatchError,
    ),

    #[error(transparent)]
    IdentityAssetLockTransactionIsNotFoundError(IdentityAssetLockTransactionIsNotFoundError),

    #[error(transparent)]
    IdentityAssetLockTransactionOutPointAlreadyConsumedError(
        IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    ),

    #[error(transparent)]
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError(
        IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
    ),

    #[error(transparent)]
    IdentityAssetLockStateTransitionReplayError(IdentityAssetLockStateTransitionReplayError),

    #[error(transparent)]
    IdentityAssetLockTransactionOutputNotFoundError(
        IdentityAssetLockTransactionOutputNotFoundError,
    ),

    #[error(transparent)]
    InvalidAssetLockProofCoreChainHeightError(InvalidAssetLockProofCoreChainHeightError),

    #[error(transparent)]
    InvalidIdentityAssetLockProofChainLockValidationError(
        InvalidIdentityAssetLockProofChainLockValidationError,
    ),

    #[error(transparent)]
    InvalidAssetLockProofTransactionHeightError(InvalidAssetLockProofTransactionHeightError),

    #[error(transparent)]
    InvalidAssetLockTransactionOutputReturnSizeError(
        InvalidAssetLockTransactionOutputReturnSizeError,
    ),

    #[error(transparent)]
    InvalidIdentityAssetLockTransactionError(InvalidIdentityAssetLockTransactionError),

    #[error(transparent)]
    InvalidIdentityAssetLockTransactionOutputError(InvalidIdentityAssetLockTransactionOutputError),

    #[error(transparent)]
    InvalidIdentityPublicKeyDataError(InvalidIdentityPublicKeyDataError),

    #[error(transparent)]
    InvalidInstantAssetLockProofError(InvalidInstantAssetLockProofError),

    #[error(transparent)]
    InvalidInstantAssetLockProofSignatureError(InvalidInstantAssetLockProofSignatureError),

    #[error(transparent)]
    MissingMasterPublicKeyError(MissingMasterPublicKeyError),

    #[error(transparent)]
    TooManyMasterPublicKeyError(TooManyMasterPublicKeyError),

    #[error(transparent)]
    MasterPublicKeyUpdateError(MasterPublicKeyUpdateError),

    #[error(transparent)]
    InvalidDocumentTypeRequiredSecurityLevelError(InvalidDocumentTypeRequiredSecurityLevelError),

    #[error(transparent)]
    InvalidIdentityPublicKeySecurityLevelError(InvalidIdentityPublicKeySecurityLevelError),

    #[error(transparent)]
    InvalidIdentityKeySignatureError(InvalidIdentityKeySignatureError),

    #[error(transparent)]
    InvalidIdentityCreditTransferAmountError(InvalidIdentityCreditTransferAmountError),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError(
        InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    ),

    #[error(transparent)]
    WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError(
        WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError,
    ),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError(
        InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    ),

    #[error(transparent)]
    InvalidIdentityCreditWithdrawalTransitionAmountError(
        InvalidIdentityCreditWithdrawalTransitionAmountError,
    ),

    #[error(transparent)]
    InvalidIdentityUpdateTransitionEmptyError(InvalidIdentityUpdateTransitionEmptyError),

    #[error(transparent)]
    InvalidIdentityUpdateTransitionDisableKeysError(
        InvalidIdentityUpdateTransitionDisableKeysError,
    ),

    #[error(transparent)]
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError(
        NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
    ),

    // State Transition
    #[error(transparent)]
    InvalidStateTransitionTypeError(InvalidStateTransitionTypeError),

    #[error(transparent)]
    MissingStateTransitionTypeError(MissingStateTransitionTypeError),

    #[error(transparent)]
    DocumentFieldMaxSizeExceededError(DocumentFieldMaxSizeExceededError),

    #[error(transparent)]
    StateTransitionMaxSizeExceededError(StateTransitionMaxSizeExceededError),

    #[error(transparent)]
    DocumentTransitionsAreAbsentError(DocumentTransitionsAreAbsentError),

    #[error(transparent)]
    IdentityCreditTransferToSelfError(IdentityCreditTransferToSelfError),

    #[error(transparent)]
    InvalidDocumentTypeNameError(InvalidDocumentTypeNameError),

    #[error(transparent)]
    IncompatibleDocumentTypeSchemaError(IncompatibleDocumentTypeSchemaError),

    #[error(transparent)]
    ContestedUniqueIndexOnMutableDocumentTypeError(ContestedUniqueIndexOnMutableDocumentTypeError),

    #[error(transparent)]
    ContestedUniqueIndexWithUniqueIndexError(ContestedUniqueIndexWithUniqueIndexError),

    #[error(transparent)]
    OverflowError(OverflowError),

    #[error(transparent)]
    UnsupportedFeatureError(UnsupportedFeatureError),

    #[error(transparent)]
    ContestedDocumentsTemporarilyNotAllowedError(ContestedDocumentsTemporarilyNotAllowedError),

    #[error(transparent)]
    DataContractTokenConfigurationUpdateError(DataContractTokenConfigurationUpdateError),

    #[error(transparent)]
    NonContiguousContractTokenPositionsError(NonContiguousContractTokenPositionsError),

    #[error(transparent)]
    NonContiguousContractGroupPositionsError(NonContiguousContractGroupPositionsError),

    #[error(transparent)]
    InvalidTokenBaseSupplyError(InvalidTokenBaseSupplyError),

    #[error(transparent)]
    InvalidTokenIdError(InvalidTokenIdError),

    #[error(transparent)]
    InvalidTokenAmountError(InvalidTokenAmountError),

    #[error(transparent)]
    InvalidTokenPositionError(InvalidTokenPositionError),

    #[error(transparent)]
    InvalidTokenConfigUpdateNoChangeError(InvalidTokenConfigUpdateNoChangeError),

    #[error(transparent)]
    InvalidTokenDistributionFunctionDivideByZeroError(
        InvalidTokenDistributionFunctionDivideByZeroError,
    ),

    #[error(transparent)]
    InvalidTokenDistributionFunctionInvalidParameterError(
        InvalidTokenDistributionFunctionInvalidParameterError,
    ),

    #[error(transparent)]
    InvalidTokenDistributionFunctionInvalidParameterTupleError(
        InvalidTokenDistributionFunctionInvalidParameterTupleError,
    ),

    #[error(transparent)]
    InvalidTokenDistributionFunctionIncoherenceError(
        InvalidTokenDistributionFunctionIncoherenceError,
    ),

    #[error(transparent)]
    TokenTransferToOurselfError(TokenTransferToOurselfError),

    #[error(transparent)]
    InvalidTokenNoteTooBigError(InvalidTokenNoteTooBigError),

    #[error(transparent)]
    ContractHasNoTokensError(ContractHasNoTokensError),

    #[error(transparent)]
    GroupPositionDoesNotExistError(GroupPositionDoesNotExistError),

    #[error(transparent)]
    InvalidActionIdError(InvalidActionIdError),

    #[error(transparent)]
    DestinationIdentityForTokenMintingNotSetError(DestinationIdentityForTokenMintingNotSetError),

    #[error(transparent)]
    ChoosingTokenMintRecipientNotAllowedError(ChoosingTokenMintRecipientNotAllowedError),

    #[error(transparent)]
    GroupActionNotAllowedOnTransitionError(GroupActionNotAllowedOnTransitionError),

    #[error(transparent)]
    GroupExceedsMaxMembersError(GroupExceedsMaxMembersError),

    #[error(transparent)]
    GroupMemberHasPowerOfZeroError(GroupMemberHasPowerOfZeroError),

    #[error(transparent)]
    GroupMemberHasPowerOverLimitError(GroupMemberHasPowerOverLimitError),

    #[error(transparent)]
    GroupTotalPowerLessThanRequiredError(GroupTotalPowerLessThanRequiredError),

    #[error(transparent)]
    GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(
        GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError,
    ),

    #[error(transparent)]
    MissingDefaultLocalizationError(MissingDefaultLocalizationError),

    #[error(transparent)]
    UnknownGasFeesPaidByError(UnknownGasFeesPaidByError),

    #[error(transparent)]
    UnknownDocumentActionTokenEffectError(UnknownDocumentActionTokenEffectError),

    #[error(transparent)]
    TokenPaymentByBurningOnlyAllowedOnInternalTokenError(
        TokenPaymentByBurningOnlyAllowedOnInternalTokenError,
    ),

    #[error(transparent)]
    TooManyKeywordsError(TooManyKeywordsError),

    #[error(transparent)]
    DuplicateKeywordsError(DuplicateKeywordsError),

    #[error(transparent)]
    InvalidKeywordLengthError(InvalidKeywordLengthError),

    #[error(transparent)]
    InvalidDescriptionLengthError(InvalidDescriptionLengthError),

    #[error(transparent)]
    NewTokensDestinationIdentityOptionRequiredError(
        NewTokensDestinationIdentityOptionRequiredError,
    ),

    #[error(transparent)]
    InvalidKeywordCharacterError(InvalidKeywordCharacterError),

    #[error(transparent)]
    InvalidTokenNameCharacterError(InvalidTokenNameCharacterError),

    #[error(transparent)]
    DecimalsOverLimitError(DecimalsOverLimitError),

    #[error(transparent)]
    InvalidTokenNameLengthError(InvalidTokenNameLengthError),

    #[error(transparent)]
    InvalidTokenLanguageCodeError(InvalidTokenLanguageCodeError),

    #[error(transparent)]
    MainGroupIsNotDefinedError(MainGroupIsNotDefinedError),

    #[error(transparent)]
    GroupRequiredPowerIsInvalidError(GroupRequiredPowerIsInvalidError),

    #[error(transparent)]
    TokenNoteOnlyAllowedWhenProposerError(TokenNoteOnlyAllowedWhenProposerError),

    #[error(transparent)]
    InvalidTokenDistributionBlockIntervalTooShortError(
        InvalidTokenDistributionBlockIntervalTooShortError,
    ),

    #[error(transparent)]
    InvalidTokenDistributionTimeIntervalTooShortError(
        InvalidTokenDistributionTimeIntervalTooShortError,
    ),

    #[error(transparent)]
    InvalidTokenDistributionTimeIntervalNotMinuteAlignedError(
        InvalidTokenDistributionTimeIntervalNotMinuteAlignedError,
    ),
    #[error(transparent)]
    RedundantDocumentPaidForByTokenWithContractId(RedundantDocumentPaidForByTokenWithContractId),

    #[error(transparent)]
    GroupHasTooFewMembersError(GroupHasTooFewMembersError),

    #[error(transparent)]
    InvalidKeyPurposeForContractBoundsError(InvalidKeyPurposeForContractBoundsError),
}

impl From<BasicError> for ConsensusError {
    fn from(error: BasicError) -> Self {
        Self::BasicError(error)
    }
}
