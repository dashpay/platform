use crate::consensus::signature::SignatureError;
#[cfg(feature = "state-transition-validation")]
use crate::consensus::state::data_trigger::DataTriggerError;

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
            // Versioning
            Self::UnsupportedVersionError(_) => 1100,
            // Decoding
            Self::ProtocolVersionParsingError { .. } => 1000,
            Self::SerializedObjectParsingError { .. } => 1001,
            Self::UnsupportedProtocolVersionError(_) => 1002,
            Self::IncompatibleProtocolVersionError(_) => 1003,
            Self::VersionError(_) => 1004,

            // Structure error
            Self::JsonSchemaCompilationError(..) => 1004,
            Self::JsonSchemaError(_) => 1005,
            Self::InvalidIdentifierError { .. } => 1006,
            Self::ValueError(_) => 1060,

            // DataContract
            Self::DataContractMaxDepthExceedError { .. } => 1007,
            Self::DuplicateIndexError { .. } => 1008,
            Self::IncompatibleRe2PatternError { .. } => 1009,
            Self::InvalidCompoundIndexError { .. } => 1010,
            Self::InvalidDataContractIdError { .. } => 1011,
            Self::InvalidIndexedPropertyConstraintError { .. } => 1012,
            Self::InvalidIndexPropertyTypeError { .. } => 1013,
            Self::InvalidJsonSchemaRefError { .. } => 1014,
            Self::SystemPropertyIndexAlreadyPresentError { .. } => 1015,
            Self::UndefinedIndexPropertyError { .. } => 1016,
            Self::UniqueIndicesLimitReachedError { .. } => 1017,
            Self::DuplicateIndexNameError { .. } => 1048,
            Self::InvalidDataContractVersionError { .. } => 1050,
            Self::IncompatibleDataContractSchemaError { .. } => 1051,
            Self::DataContractImmutablePropertiesUpdateError { .. } => 1052,
            Self::DataContractUniqueIndicesChangedError { .. } => 1053,
            Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 1054,
            Self::DataContractHaveNewUniqueIndexError { .. } => 1055,

            // Document
            Self::DataContractNotPresentError { .. } => 1018,
            Self::DuplicateDocumentTransitionsWithIdsError { .. } => 1019,
            Self::DuplicateDocumentTransitionsWithIndicesError { .. } => 1020,
            Self::InconsistentCompoundIndexDataError { .. } => 1021,
            Self::InvalidDocumentTransitionActionError { .. } => 1022,
            Self::InvalidDocumentTransitionIdError { .. } => 1023,
            Self::InvalidDocumentTypeError { .. } => 1024,
            Self::MissingDataContractIdBasicError { .. } => 1025,
            Self::MissingDocumentTransitionActionError { .. } => 1026,
            Self::MissingDocumentTransitionTypeError { .. } => 1027,
            Self::MissingDocumentTypeError { .. } => 1028,
            Self::MissingPositionsInDocumentTypePropertiesError { .. } => 1067,
            Self::MaxDocumentsTransitionsExceededError { .. } => 1065,
            Self::DocumentTransitionsAreAbsentError { .. } => 1068,

            // Identity
            Self::DuplicatedIdentityPublicKeyBasicError(_) => 1029,
            Self::DuplicatedIdentityPublicKeyIdBasicError(_) => 1030,
            Self::IdentityAssetLockProofLockedTransactionMismatchError(_) => 1031,
            Self::IdentityAssetLockTransactionIsNotFoundError(_) => 1032,
            Self::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => 1033,
            Self::IdentityAssetLockTransactionOutputNotFoundError(_) => 1034,
            Self::InvalidAssetLockProofCoreChainHeightError(_) => 1035,
            Self::InvalidAssetLockProofTransactionHeightError(_) => 1036,
            Self::InvalidAssetLockTransactionOutputReturnSizeError(_) => 1037,
            Self::InvalidIdentityAssetLockTransactionError(_) => 1038,
            Self::InvalidIdentityAssetLockTransactionOutputError(_) => 1039,
            Self::InvalidIdentityPublicKeyDataError(_) => 1040,
            Self::InvalidInstantAssetLockProofError(_) => 1041,
            Self::InvalidInstantAssetLockProofSignatureError(_) => 1042,
            Self::InvalidIdentityAssetLockProofChainLockValidationError(_) => 1043,
            Self::DataContractBoundsNotPresentError(_) => 1066,

            Self::MissingMasterPublicKeyError(_) => 1046,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
            Self::InvalidIdentityKeySignatureError { .. } => 1056,
            Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 1057,
            Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 1058,
            Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 1059,
            Self::InvalidIdentityCreditTransferAmountError(_) => 1061,
            Self::InvalidIdentityCreditWithdrawalTransitionAmountError(_) => 1062,
            Self::InvalidIdentityUpdateTransitionEmptyError(_) => 1063,
            Self::InvalidIdentityUpdateTransitionDisableKeysError(_) => 1064,
            Self::IdentityCreditTransferToSelfError(_) => 1070,

            // State Transition
            Self::InvalidStateTransitionTypeError { .. } => 1043,
            Self::MissingStateTransitionTypeError { .. } => 1044,
            Self::StateTransitionMaxSizeExceededError { .. } => 1045,
        }
    }
}

impl ErrorWithCode for SignatureError {
    fn code(&self) -> u32 {
        match self {
            Self::IdentityNotFoundError { .. } => 2000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 2001,
            Self::InvalidStateTransitionSignatureError { .. } => 2002,
            Self::MissingPublicKeyError { .. } => 2003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 2004,
            Self::WrongPublicKeyPurposeError { .. } => 2005,
            Self::PublicKeyIsDisabledError { .. } => 2006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 2007,
            Self::SignatureShouldNotBePresentError(_) => 2008,
            Self::BasicECDSAError(_) => 2009,
            Self::BasicBLSError(_) => 2010,
        }
    }
}

impl ErrorWithCode for FeeError {
    fn code(&self) -> u32 {
        match self {
            Self::BalanceIsNotEnoughError { .. } => 3000,
        }
    }
}

impl ErrorWithCode for StateError {
    fn code(&self) -> u32 {
        match self {
            // Data contract
            Self::DataContractAlreadyPresentError { .. } => 4000,
            Self::DataContractIsReadonlyError { .. } => 4026,
            #[cfg(feature = "validation")]
            Self::DataTriggerError(ref e) => e.code(),
            Self::DataContractConfigUpdateError { .. } => 4027,

            // Document
            Self::DocumentAlreadyPresentError { .. } => 4004,
            Self::DocumentNotFoundError { .. } => 4005,
            Self::DocumentOwnerIdMismatchError { .. } => 4006,
            Self::DocumentTimestampsMismatchError { .. } => 4007,
            Self::DocumentTimestampWindowViolationError { .. } => 4008,
            Self::DuplicateUniqueIndexError { .. } => 4009,
            Self::InvalidDocumentRevisionError { .. } => 4010,
            Self::DocumentTimestampsAreEqualError(_) => 4025,

            // Identity
            Self::IdentityAlreadyExistsError(_) => 4011,
            Self::IdentityPublicKeyDisabledAtWindowViolationError { .. } => 4012,
            Self::IdentityPublicKeyIsReadOnlyError { .. } => 4017,
            Self::InvalidIdentityPublicKeyIdError { .. } => 4018,
            Self::InvalidIdentityRevisionError { .. } => 4019,
            Self::MaxIdentityPublicKeyLimitReachedError { .. } => 4020,
            Self::DuplicatedIdentityPublicKeyStateError { .. } => 4021,
            Self::DuplicatedIdentityPublicKeyIdStateError { .. } => 4022,
            Self::IdentityPublicKeyIsDisabledError { .. } => 4023,
            Self::MissingIdentityPublicKeyIdsError { .. } => 4024,
            Self::IdentityInsufficientBalanceError(_) => 4026,
            Self::IdentityPublicKeyAlreadyExistsForUniqueContractBoundsError(_) => 4027,
            Self::InvalidAssetLockProofValueError(_) => 4028,
        }
    }
}

#[cfg(feature = "state-transition-validation")]
impl ErrorWithCode for DataTriggerError {
    fn code(&self) -> u32 {
        match self {
            Self::DataTriggerConditionError { .. } => 4001,
            Self::DataTriggerExecutionError { .. } => 4002,
            Self::DataTriggerInvalidResultError { .. } => 4003,
        }
    }
}
