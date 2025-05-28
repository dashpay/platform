use crate::consensus::signature::SignatureError;
use crate::consensus::state::data_trigger::DataTriggerError;
use crate::data_contract::errors::DataContractError;

use crate::errors::consensus::{
    basic::BasicError, fee::fee_error::FeeError, state::state_error::StateError, ConsensusError,
};

pub trait ErrorWithCode {
    /// Returns the error code
    fn code(&self) -> u32;
}

impl ErrorWithCode for ConsensusError {
    fn code(&self) -> u32 {
        match self {
            Self::BasicError(e) => e.code(),
            Self::SignatureError(e) => e.code(),
            Self::StateError(e) => e.code(),
            Self::FeeError(e) => e.code(),

            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::DefaultError => 1, // this should never happen
        }
    }
}
impl ErrorWithCode for BasicError {
    fn code(&self) -> u32 {
        match self {
            // Versioning Errors: 10000-10099
            Self::UnsupportedVersionError(_) => 10000,
            Self::ProtocolVersionParsingError { .. } => 10001,
            Self::SerializedObjectParsingError { .. } => 10002,
            Self::UnsupportedProtocolVersionError(_) => 10003,
            Self::IncompatibleProtocolVersionError(_) => 10004,
            Self::VersionError(_) => 10005,
            Self::UnsupportedFeatureError(_) => 10006,

            // Structure Errors: 10100-10199
            Self::JsonSchemaCompilationError(..) => 10100,
            Self::JsonSchemaError(_) => 10101,
            Self::InvalidIdentifierError { .. } => 10102,
            Self::ValueError(_) => 10103,

            // DataContract Errors: 10200-10349
            Self::DataContractMaxDepthExceedError { .. } => 10200,
            Self::DuplicateIndexError { .. } => 10201,
            Self::IncompatibleRe2PatternError { .. } => 10202,
            Self::InvalidCompoundIndexError { .. } => 10203,
            Self::InvalidDataContractIdError { .. } => 10204,
            Self::InvalidIndexedPropertyConstraintError { .. } => 10205,
            Self::InvalidIndexPropertyTypeError { .. } => 10206,
            Self::InvalidJsonSchemaRefError { .. } => 10207,
            Self::SystemPropertyIndexAlreadyPresentError { .. } => 10208,
            Self::UndefinedIndexPropertyError { .. } => 10209,
            Self::UniqueIndicesLimitReachedError { .. } => 10210,
            Self::DuplicateIndexNameError { .. } => 10211,
            Self::InvalidDataContractVersionError { .. } => 10212,
            Self::IncompatibleDataContractSchemaError { .. } => 10213,
            Self::ContractError(DataContractError::DocumentTypesAreMissingError { .. }) => 10214,
            Self::DataContractImmutablePropertiesUpdateError { .. } => 10215,
            Self::DataContractUniqueIndicesChangedError { .. } => 10216,
            Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 10217,
            Self::DataContractHaveNewUniqueIndexError { .. } => 10218,
            Self::InvalidDocumentTypeRequiredSecurityLevelError { .. } => 10219,
            Self::UnknownSecurityLevelError { .. } => 10220,
            Self::UnknownStorageKeyRequirementsError { .. } => 10221,
            Self::ContractError(DataContractError::DecodingContractError { .. }) => 10222,
            Self::ContractError(DataContractError::DecodingDocumentError { .. }) => 10223,
            Self::ContractError(DataContractError::InvalidDocumentTypeError { .. }) => 10224,
            Self::ContractError(DataContractError::MissingRequiredKey(_)) => 10225,
            Self::ContractError(DataContractError::FieldRequirementUnmet(_)) => 10226,
            Self::ContractError(DataContractError::KeyWrongType(_)) => 10227,
            Self::ContractError(DataContractError::ValueWrongType(_)) => 10228,
            Self::ContractError(DataContractError::ValueDecodingError(_)) => 10229,
            Self::ContractError(DataContractError::EncodingDataStructureNotSupported(_)) => 10230,
            Self::ContractError(DataContractError::InvalidContractStructure(_)) => 10231,
            Self::ContractError(DataContractError::DocumentTypeNotFound(_)) => 10232,
            Self::ContractError(DataContractError::DocumentTypeFieldNotFound(_)) => 10233,
            Self::ContractError(DataContractError::ReferenceDefinitionNotFound(_)) => 10234,
            Self::ContractError(DataContractError::DocumentOwnerIdMissing(_)) => 10235,
            Self::ContractError(DataContractError::DocumentIdMissing(_)) => 10236,
            Self::ContractError(DataContractError::Unsupported(_)) => 10237,
            Self::ContractError(DataContractError::CorruptedSerialization(_)) => 10238,
            Self::ContractError(DataContractError::JsonSchema(_)) => 10239,
            Self::ContractError(DataContractError::InvalidURI(_)) => 10240,
            Self::ContractError(DataContractError::KeyWrongBounds(_)) => 10241,
            Self::ContractError(DataContractError::KeyValueMustExist(_)) => 10242,
            Self::UnknownTransferableTypeError { .. } => 10243,
            Self::UnknownTradeModeError { .. } => 10244,
            Self::UnknownDocumentCreationRestrictionModeError { .. } => 10245,
            Self::IncompatibleDocumentTypeSchemaError { .. } => 10246,
            Self::ContractError(DataContractError::RegexError(_)) => 10247,
            Self::ContestedUniqueIndexOnMutableDocumentTypeError(_) => 10248,
            Self::ContestedUniqueIndexWithUniqueIndexError(_) => 10249,
            Self::DataContractTokenConfigurationUpdateError { .. } => 10250,
            Self::InvalidTokenBaseSupplyError(_) => 10251,
            Self::NonContiguousContractGroupPositionsError(_) => 10252,
            Self::NonContiguousContractTokenPositionsError(_) => 10253,
            Self::InvalidTokenDistributionFunctionDivideByZeroError(_) => 10254,
            Self::InvalidTokenDistributionFunctionInvalidParameterError(_) => 10255,
            Self::InvalidTokenDistributionFunctionInvalidParameterTupleError(_) => 10256,
            Self::InvalidTokenDistributionFunctionIncoherenceError(_) => 10257,
            Self::MissingDefaultLocalizationError(_) => 10258,
            Self::UnknownGasFeesPaidByError(_) => 10259,
            Self::UnknownDocumentActionTokenEffectError(_) => 10260,
            Self::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(_) => 10261,
            Self::TooManyKeywordsError(_) => 10262,
            Self::DuplicateKeywordsError(_) => 10263,
            Self::InvalidDescriptionLengthError(_) => 10264,
            Self::NewTokensDestinationIdentityOptionRequiredError(_) => 10265,
            Self::InvalidTokenNameCharacterError(_) => 10266,
            Self::InvalidTokenNameLengthError(_) => 10267,
            Self::InvalidTokenLanguageCodeError(_) => 10268,
            Self::InvalidKeywordCharacterError(_) => 10269,
            Self::InvalidKeywordLengthError(_) => 10270,
            Self::DecimalsOverLimitError(_) => 10271,
            Self::RedundantDocumentPaidForByTokenWithContractId(_) => 10272,

            // Group Errors: 10350-10399
            Self::GroupPositionDoesNotExistError(_) => 10350,
            Self::GroupActionNotAllowedOnTransitionError(_) => 10351,
            Self::GroupTotalPowerLessThanRequiredError(_) => 10352,
            Self::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(_) => 10353,
            Self::GroupExceedsMaxMembersError(_) => 10354,
            Self::GroupMemberHasPowerOfZeroError(_) => 10355,
            Self::GroupMemberHasPowerOverLimitError(_) => 10356,
            Self::MainGroupIsNotDefinedError(_) => 10357,
            Self::GroupRequiredPowerIsInvalidError(_) => 10358,
            Self::GroupHasTooFewMembersError(_) => 10359,

            // Document Errors: 10400-10449
            Self::DataContractNotPresentError { .. } => 10400,
            Self::DuplicateDocumentTransitionsWithIdsError { .. } => 10401,
            Self::DuplicateDocumentTransitionsWithIndicesError { .. } => 10402,
            Self::InconsistentCompoundIndexDataError { .. } => 10403,
            Self::InvalidDocumentTransitionActionError { .. } => 10404,
            Self::InvalidDocumentTransitionIdError { .. } => 10405,
            Self::InvalidDocumentTypeError { .. } => 10406,
            Self::MissingDataContractIdBasicError { .. } => 10407,
            Self::MissingDocumentTransitionActionError { .. } => 10408,
            Self::MissingDocumentTransitionTypeError { .. } => 10409,
            Self::MissingDocumentTypeError { .. } => 10410,
            Self::MissingPositionsInDocumentTypePropertiesError { .. } => 10411,
            Self::MaxDocumentsTransitionsExceededError { .. } => 10412,
            Self::DocumentTransitionsAreAbsentError { .. } => 10413,
            Self::NonceOutOfBoundsError(_) => 10414,
            Self::InvalidDocumentTypeNameError(_) => 10415,
            Self::DocumentCreationNotAllowedError(_) => 10416,
            Self::DocumentFieldMaxSizeExceededError(_) => 10417,
            Self::ContestedDocumentsTemporarilyNotAllowedError(_) => 10418,

            // Token Errors: 10450-10499
            Self::InvalidTokenIdError(_) => 10450,
            Self::InvalidTokenPositionError(_) => 10451,
            Self::InvalidActionIdError(_) => 10452,
            Self::ContractHasNoTokensError(_) => 10453,
            Self::DestinationIdentityForTokenMintingNotSetError(_) => 10454,
            Self::ChoosingTokenMintRecipientNotAllowedError(_) => 10455,
            Self::TokenTransferToOurselfError(_) => 10456,
            Self::InvalidTokenConfigUpdateNoChangeError(_) => 10457,
            Self::InvalidTokenAmountError(_) => 10458,
            Self::InvalidTokenNoteTooBigError(_) => 10459,
            Self::TokenNoteOnlyAllowedWhenProposerError(_) => 10460,

            // Identity Errors: 10500-10599
            Self::DuplicatedIdentityPublicKeyBasicError(_) => 10500,
            Self::DuplicatedIdentityPublicKeyIdBasicError(_) => 10501,
            Self::IdentityAssetLockProofLockedTransactionMismatchError(_) => 10502,
            Self::IdentityAssetLockTransactionIsNotFoundError(_) => 10503,
            Self::IdentityAssetLockTransactionOutPointAlreadyConsumedError(_) => 10504,
            Self::IdentityAssetLockTransactionOutputNotFoundError(_) => 10505,
            Self::InvalidAssetLockProofCoreChainHeightError(_) => 10506,
            Self::InvalidAssetLockProofTransactionHeightError(_) => 10507,
            Self::InvalidAssetLockTransactionOutputReturnSizeError(_) => 10508,
            Self::InvalidIdentityAssetLockTransactionError(_) => 10509,
            Self::InvalidIdentityAssetLockTransactionOutputError(_) => 10510,
            Self::InvalidIdentityPublicKeyDataError(_) => 10511,
            Self::InvalidInstantAssetLockProofError(_) => 10512,
            Self::InvalidInstantAssetLockProofSignatureError(_) => 10513,
            Self::InvalidIdentityAssetLockProofChainLockValidationError(_) => 10514,
            Self::DataContractBoundsNotPresentError(_) => 10515,
            Self::DisablingKeyIdAlsoBeingAddedInSameTransitionError(_) => 10516,
            Self::MissingMasterPublicKeyError(_) => 10517,
            Self::TooManyMasterPublicKeyError(_) => 10518,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 10519,
            Self::InvalidIdentityKeySignatureError { .. } => 10520,
            Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 10521,
            Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 10522,
            Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 10523,
            Self::InvalidIdentityCreditTransferAmountError(_) => 10524,
            Self::InvalidIdentityCreditWithdrawalTransitionAmountError(_) => 10525,
            Self::InvalidIdentityUpdateTransitionEmptyError(_) => 10526,
            Self::InvalidIdentityUpdateTransitionDisableKeysError(_) => 10527,
            Self::IdentityCreditTransferToSelfError(_) => 10528,
            Self::MasterPublicKeyUpdateError(_) => 10529,
            Self::IdentityAssetLockTransactionOutPointNotEnoughBalanceError(_) => 10530,
            Self::IdentityAssetLockStateTransitionReplayError(_) => 10531,
            Self::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError(_) => 10532,

            // State Transition Errors: 10600-10699
            Self::InvalidStateTransitionTypeError { .. } => 10600,
            Self::MissingStateTransitionTypeError { .. } => 10601,
            Self::StateTransitionMaxSizeExceededError { .. } => 10602,

            // General Errors 10700-10799
            Self::OverflowError(_) => 10700,
        }
    }
}

impl ErrorWithCode for SignatureError {
    fn code(&self) -> u32 {
        match self {
            Self::IdentityNotFoundError { .. } => 20000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 20001,
            Self::InvalidStateTransitionSignatureError { .. } => 20002,
            Self::MissingPublicKeyError { .. } => 20003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 20004,
            Self::WrongPublicKeyPurposeError { .. } => 20005,
            Self::PublicKeyIsDisabledError { .. } => 20006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 20007,
            Self::SignatureShouldNotBePresentError(_) => 20008,
            Self::BasicECDSAError(_) => 20009,
            Self::BasicBLSError(_) => 20010,
            Self::InvalidSignaturePublicKeyPurposeError(_) => 20011,
        }
    }
}

impl ErrorWithCode for FeeError {
    fn code(&self) -> u32 {
        match self {
            Self::BalanceIsNotEnoughError { .. } => 30000,
        }
    }
}

impl ErrorWithCode for StateError {
    fn code(&self) -> u32 {
        match self {
            // Data contract Errors: 40000-40099
            Self::DataContractAlreadyPresentError { .. } => 40000,
            Self::DataContractIsReadonlyError { .. } => 40001,
            Self::DataContractConfigUpdateError { .. } => 40002,
            Self::DataContractUpdatePermissionError(_) => 40003,
            Self::DataContractUpdateActionNotAllowedError(_) => 40004,
            Self::PreProgrammedDistributionTimestampInPastError(_) => 40005,
            Self::IdentityInTokenConfigurationNotFoundError(_) => 40006,
            Self::IdentityMemberOfGroupNotFoundError(_) => 40007,
            Self::DataContractNotFoundError(_) => 40008,
            Self::InvalidTokenPositionStateError(_) => 40009,

            // Document Errors: 40100-40199
            Self::DocumentAlreadyPresentError { .. } => 40100,
            Self::DocumentNotFoundError { .. } => 40101,
            Self::DocumentOwnerIdMismatchError { .. } => 40102,
            Self::DocumentTimestampsMismatchError { .. } => 40103,
            Self::DocumentTimestampWindowViolationError { .. } => 40104,
            Self::DuplicateUniqueIndexError { .. } => 40105,
            Self::InvalidDocumentRevisionError { .. } => 40106,
            Self::DocumentTimestampsAreEqualError(_) => 40107,
            Self::DocumentNotForSaleError(_) => 40108,
            Self::DocumentIncorrectPurchasePriceError(_) => 40109,
            Self::DocumentContestCurrentlyLockedError(_) => 40110,
            Self::DocumentContestNotJoinableError(_) => 40111,
            Self::DocumentContestIdentityAlreadyContestantError(_) => 40112,
            Self::DocumentContestDocumentWithSameIdAlreadyPresentError(_) => 40113,
            Self::DocumentContestNotPaidForError(_) => 40114,
            Self::RequiredTokenPaymentInfoNotSetError(_) => 40115,
            Self::IdentityHasNotAgreedToPayRequiredTokenAmountError(_) => 40116,
            Self::IdentityTryingToPayWithWrongTokenError(_) => 40117,

            // Identity Errors: 40200-40299
            Self::IdentityAlreadyExistsError(_) => 40200,
            Self::IdentityPublicKeyIsReadOnlyError { .. } => 40201,
            Self::InvalidIdentityPublicKeyIdError { .. } => 40202,
            Self::InvalidIdentityRevisionError { .. } => 40203,
            Self::InvalidIdentityNonceError(_) => 40204,
            Self::MaxIdentityPublicKeyLimitReachedError { .. } => 40205,
            Self::DuplicatedIdentityPublicKeyStateError { .. } => 40206,
            Self::DuplicatedIdentityPublicKeyIdStateError { .. } => 40207,
            Self::IdentityPublicKeyIsDisabledError { .. } => 40208,
            Self::MissingIdentityPublicKeyIdsError { .. } => 40209,
            Self::IdentityInsufficientBalanceError(_) => 40210,
            Self::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(_) => 40211,
            Self::DocumentTypeUpdateError(_) => 40212,
            Self::MissingTransferKeyError(_) => 40214,
            Self::NoTransferKeyForCoreWithdrawalAvailableError(_) => 40215,
            Self::RecipientIdentityDoesNotExistError(_) => 40216,
            Self::IdentityToFreezeDoesNotExistError(_) => 40217,

            // Voting Errors: 40300-40399
            Self::MasternodeNotFoundError(_) => 40300,
            Self::VotePollNotFoundError(_) => 40301,
            Self::VotePollNotAvailableForVotingError(_) => 40302,
            Self::MasternodeVotedTooManyTimesError(_) => 40303,
            Self::MasternodeVoteAlreadyPresentError(_) => 40304,
            Self::MasternodeIncorrectVotingAddressError(_) => 40305,
            Self::MasternodeIncorrectVoterIdentityIdError(_) => 40306,

            // Prefunded specialized balances Errors: 40400-40499
            Self::PrefundedSpecializedBalanceInsufficientError(_) => 40400,
            Self::PrefundedSpecializedBalanceNotFoundError(_) => 40401,

            // Data trigger errors: 40500-40699
            Self::DataTriggerError(ref e) => e.code(),

            // Token errors: 40700-40799
            Self::IdentityDoesNotHaveEnoughTokenBalanceError(_) => 40700,
            Self::UnauthorizedTokenActionError(_) => 40701,
            Self::IdentityTokenAccountFrozenError(_) => 40702,
            Self::IdentityTokenAccountNotFrozenError(_) => 40703,
            Self::TokenSettingMaxSupplyToLessThanCurrentSupplyError(_) => 40704,
            Self::TokenMintPastMaxSupplyError(_) => 40705,
            Self::NewTokensDestinationIdentityDoesNotExistError(_) => 40706,
            Self::NewAuthorizedActionTakerIdentityDoesNotExistError(_) => 40707,
            Self::NewAuthorizedActionTakerGroupDoesNotExistError(_) => 40708,
            Self::NewAuthorizedActionTakerMainGroupNotSetError(_) => 40709,
            Self::InvalidGroupPositionError(_) => 40710,
            Self::TokenIsPausedError(_) => 40711,
            Self::IdentityTokenAccountAlreadyFrozenError(_) => 40712,
            Self::TokenAlreadyPausedError(_) => 40713,
            Self::TokenNotPausedError(_) => 40714,
            Self::InvalidTokenClaimPropertyMismatch(_) => 40715,
            Self::InvalidTokenClaimNoCurrentRewards(_) => 40716,
            Self::InvalidTokenClaimWrongClaimant(_) => 40717,
            Self::TokenTransferRecipientIdentityNotExistError(_) => 40718,
            Self::TokenDirectPurchaseUserPriceTooLow(_) => 40719,
            Self::TokenAmountUnderMinimumSaleAmount(_) => 40720,
            Self::TokenNotForDirectSale(_) => 40721,

            // Group errors: 40800-40899
            Self::IdentityNotMemberOfGroupError(_) => 40800,
            Self::GroupActionDoesNotExistError(_) => 40801,
            Self::GroupActionAlreadyCompletedError(_) => 40802,
            Self::GroupActionAlreadySignedByIdentityError(_) => 40803,
            Self::ModificationOfGroupActionMainParametersNotPermittedError(_) => 40804,
        }
    }
}

impl ErrorWithCode for DataTriggerError {
    fn code(&self) -> u32 {
        match self {
            Self::DataTriggerConditionError { .. } => 40500,
            Self::DataTriggerExecutionError { .. } => 40501,
            Self::DataTriggerInvalidResultError { .. } => 40502,
        }
    }
}
