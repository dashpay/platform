use crate::consensus::state::data_contract::data_trigger::data_trigger_error::DataTriggerError;

use crate::errors::consensus::{
    basic::BasicError, fee::fee_error::FeeError, signature::signature_error::SignatureError,
    state::state_error::StateError, ConsensusError,
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
            ConsensusError::TestConsensusError(_) => 666,
        }
    }
}

impl ErrorWithCode for BasicError {
    fn code(&self) -> u32 {
        match self {
            // Decoding
            Self::ProtocolVersionParsingError { .. } => 1000,
            Self::SerializedObjectParsingError { .. } => 1001,
            Self::UnsupportedProtocolVersionError(_) => 1002,
            Self::IncompatibleProtocolVersionError(_) => 1003,

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
            Self::MissingDocumentTypeError => 1028,

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
            Self::MissingMasterPublicKeyError(_) => 1046,
            Self::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
            Self::InvalidIdentityKeySignatureError { .. } => 1056,
            Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 1057,
            Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 1058,
            Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 1059,

            // State Transition
            Self::InvalidStateTransitionTypeError { .. } => 1043,
            Self::MissingStateTransitionTypeError => 1044,
            Self::StateTransitionMaxSizeExceededError { .. } => 1045,
        }
    }
}

impl ErrorWithCode for SignatureError {
    fn code(&self) -> u32 {
        match self {
            Self::IdentityNotFoundError { .. } => 2000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 2001,
            Self::InvalidStateTransitionSignatureError => 2002,
            Self::MissingPublicKeyError { .. } => 2003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 2004,
            Self::WrongPublicKeyPurposeError { .. } => 2005,
            Self::PublicKeyIsDisabledError { .. } => 2006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 2007,
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

            Self::DataTriggerError(ref e) => e.code(),

            // Document
            Self::DocumentAlreadyPresentError { .. } => 4004,
            Self::DocumentNotFoundError { .. } => 4005,
            Self::DocumentOwnerIdMismatchError { .. } => 4006,
            Self::DocumentTimestampsMismatchError { .. } => 4007,
            Self::DocumentTimestampWindowViolationError { .. } => 4008,
            Self::DuplicateUniqueIndexError { .. } => 4009,
            Self::InvalidDocumentRevisionError { .. } => 4010,

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
            Self::IdentityInsufficientBalanceError(_) => 4024,
        }
    }
}

impl ErrorWithCode for DataTriggerError {
    fn code(&self) -> u32 {
        match self {
            Self::DataTriggerConditionError { .. } => 4001,
            Self::DataTriggerExecutionError { .. } => 4002,
            Self::DataTriggerInvalidResultError { .. } => 4003,
        }
    }
}

// pub fn create_consensus_error_from_code(code: u32, args: Value) -> Result<ConsensusError, ProtocolError> {
//     /*
//         // Decoding
//     1000: ProtocolVersionParsingError,
//     1001: SerializedObjectParsingError,
//
//     // General
//     1002: UnsupportedProtocolVersionError,
//     1003: IncompatibleProtocolVersionError,
//     1004: JsonSchemaCompilationError,
//     1005: JsonSchemaError,
//     1006: InvalidIdentifierError,
//
//     // Data Contract
//     1007: DataContractMaxDepthExceedError,
//     1008: DuplicateIndexError,
//     1009: IncompatibleRe2PatternError,
//     1010: InvalidCompoundIndexError,
//     1011: InvalidDataContractIdError,
//     1012: InvalidIndexedPropertyConstraintError,
//     1013: InvalidIndexPropertyTypeError,
//     1014: InvalidJsonSchemaRefError,
//     1015: SystemPropertyIndexAlreadyPresentError,
//     1016: UndefinedIndexPropertyError,
//     1017: UniqueIndicesLimitReachedError,
//     1048: DuplicateIndexNameError,
//     1050: InvalidDataContractVersionError,
//     1051: IncompatibleDataContractSchemaError,
//     1052: DataContractImmutablePropertiesUpdateError,
//     1053: DataContractUniqueIndicesChangedError,
//     1054: DataContractInvalidIndexDefinitionUpdateError,
//     1055: DataContractHaveNewUniqueIndexError,
//
//     // Document
//     1018: DataContractNotPresentError,
//     1019: DuplicateDocumentTransitionsWithIdsError,
//     1020: DuplicateDocumentTransitionsWithIndicesError,
//     1021: InconsistentCompoundIndexDataError,
//     1022: InvalidDocumentTransitionActionError,
//     1023: InvalidDocumentTransitionIdError,
//     1024: InvalidDocumentTypeError,
//     1025: MissingDataContractIdError,
//     1026: MissingDocumentTransitionActionError,
//     1027: MissingDocumentTransitionTypeError,
//     1028: MissingDocumentTypeError,
//
//     // Identity
//     1029: DuplicatedIdentityPublicKeyError,
//     1030: DuplicatedIdentityPublicKeyIdError,
//     1031: IdentityAssetLockProofLockedTransactionMismatchError,
//     1032: IdentityAssetLockTransactionIsNotFoundError,
//     1033: IdentityAssetLockTransactionOutPointAlreadyExistsError,
//     1034: IdentityAssetLockTransactionOutputNotFoundError,
//     1035: InvalidAssetLockProofCoreChainHeightError,
//     1036: InvalidAssetLockProofTransactionHeightError,
//     1037: InvalidAssetLockTransactionOutputReturnSizeError,
//     1038: InvalidIdentityAssetLockTransactionError,
//     1039: InvalidIdentityAssetLockTransactionOutputError,
//     1040: InvalidIdentityPublicKeyDataError,
//     1041: InvalidInstantAssetLockProofError,
//     1042: InvalidInstantAssetLockProofSignatureError,
//     1046: MissingMasterPublicKeyError,
//     1047: InvalidIdentityPublicKeySecurityLevelError,
//     1056: InvalidIdentityKeySignatureError,
//
//     // State Transition
//     1043: InvalidStateTransitionTypeError,
//     1044: MissingStateTransitionTypeError,
//     1045: StateTransitionMaxSizeExceededError,
//
//     /**
//      * Signature
//      */
//
//     2000: IdentityNotFoundError,
//     2001: InvalidIdentityPublicKeyTypeError,
//     2002: InvalidStateTransitionSignatureError,
//     2003: MissingPublicKeyError,
//     2004: InvalidSignaturePublicKeySecurityLevelError,
//     2005: WrongPublicKeyPurposeError,
//     2006: PublicKeyIsDisabledError,
//     2007: PublicKeySecurityLevelNotMetError,
//
//     /**
//      * Fee
//      */
//
//     3000: BalanceIsNotEnoughError,
//
//     /**
//      * State
//      */
//
//     // Data Contract
//     4000: DataContractAlreadyPresentError,
//     4001: DataTriggerConditionError,
//     4002: DataTriggerExecutionError,
//     4003: DataTriggerInvalidResultError,
//
//     // Document
//     4004: DocumentAlreadyPresentError,
//     4005: DocumentNotFoundError,
//     4006: DocumentOwnerIdMismatchError,
//     4007: DocumentTimestampsMismatchError,
//     4008: DocumentTimestampWindowViolationError,
//     4009: DuplicateUniqueIndexError,
//     4010: InvalidDocumentRevisionError,
//
//     // Identity
//     4011: IdentityAlreadyExistsError,
//     4012: IdentityPublicKeyDisabledAtWindowViolationError,
//     4017: IdentityPublicKeyIsReadOnlyError,
//     4018: InvalidIdentityPublicKeyIdError,
//     4019: InvalidIdentityRevisionError,
//     4020: StateMaxIdentityPublicKeyLimitReachedError,
//     4021: DuplicatedIdentityPublicKeyStateError,
//     4022: DuplicatedIdentityPublicKeyIdStateError,
//     4023: IdentityPublicKeyIsDisabledError,
//        */
//     let args_array = args.as_array()
//         .ok_or(|| ProtocolError::ValueError(ValueError::StructureError("args must be an array".into())))?;
//
//     let consensus_error = match code {
//         1000...1999 => ConsensusError::BasicError(create_basic_consensus_error_from_code(code, args_array)?),
//         1002 => ConsensusError::BasicError(BasicError::UnsupportedProtocolVersionError(args.try_into()?)),
//         2000...2999 => ConsensusError::SignatureError(create_signature_consensus_error_from_code(code, args_array)?),
//         3000...3999 => ConsensusError::FeeError(create_fee_consensus_error_from_code(code, args_array)?),
//         4000...4999 => ConsensusError::StateError(create_state_consensus_error_from_code(code, args_array)?),
//         _ => Err(ProtocolError::Error(anyhow!("invalid error code {}", code)))?,
//     };
//
//     Ok(consensus_error)
// }
//
// fn create_basic_consensus_error_from_code(code: u32, args: &Vec<Value>) -> Result<BasicError, ProtocolError> {
//    let error = match code {
//         // Decoding
//         1000 => {
//             let parsing_error = create_protocol_anyhow_error(args)?;
//
//             BasicError::ProtocolVersionParsingError(ProtocolVersionParsingError::new(parsing_error))
//         },
//         1001 => {
//            let parsing_error = create_protocol_anyhow_error(args)?;
//
//            BasicError::SerializedObjectParsingError(SerializedObjectParsingError::new(parsing_error)),
//         },
//         1002 => {
//             let parsed_protocol_version = get_index(args, 0)?.as_integer()
//                 .ok_or(ProtocolError::ValueError(ValueError::StructureError(format!("version must be u32"))))?;
//
//             let latest_version = get_index(args, 1)?.as_integer()
//                 .ok_or(ProtocolError::ValueError(ValueError::StructureError(format!("version must be u32"))))?;
//
//             BasicError::UnsupportedProtocolVersionError(UnsupportedProtocolVersionError::new(parsed_protocol_version, latest_version));
//         },
//         Self::IncompatibleProtocolVersionError(_) => 1003,
//
//         // Structure error
//         Self::JsonSchemaCompilationError(..) => 1004,
//         Self::JsonSchemaError(_) => 1005,
//         Self::InvalidIdentifierError { .. } => 1006,
//         Self::ValueError(_) => 1060,
//
//         // DataContract
//         Self::DataContractMaxDepthExceedError { .. } => 1007,
//         Self::DuplicateIndexError { .. } => 1008,
//         Self::IncompatibleRe2PatternError { .. } => 1009,
//         Self::InvalidCompoundIndexError { .. } => 1010,
//         Self::InvalidDataContractIdError { .. } => 1011,
//         Self::InvalidIndexedPropertyConstraintError { .. } => 1012,
//         Self::InvalidIndexPropertyTypeError { .. } => 1013,
//         Self::InvalidJsonSchemaRefError { .. } => 1014,
//         Self::SystemPropertyIndexAlreadyPresentError { .. } => 1015,
//         Self::UndefinedIndexPropertyError { .. } => 1016,
//         Self::UniqueIndicesLimitReachedError { .. } => 1017,
//         Self::DuplicateIndexNameError { .. } => 1048,
//         Self::InvalidDataContractVersionError { .. } => 1050,
//         Self::IncompatibleDataContractSchemaError { .. } => 1051,
//         Self::DataContractImmutablePropertiesUpdateError { .. } => 1052,
//         Self::DataContractUniqueIndicesChangedError { .. } => 1053,
//         Self::DataContractInvalidIndexDefinitionUpdateError { .. } => 1054,
//         Self::DataContractHaveNewUniqueIndexError { .. } => 1055,
//
//         // Document
//         Self::DataContractNotPresentError { .. } => 1018,
//         Self::DuplicateDocumentTransitionsWithIdsError { .. } => 1019,
//         Self::DuplicateDocumentTransitionsWithIndicesError { .. } => 1020,
//         Self::InconsistentCompoundIndexDataError { .. } => 1021,
//         Self::InvalidDocumentTransitionActionError { .. } => 1022,
//         Self::InvalidDocumentTransitionIdError { .. } => 1023,
//         Self::InvalidDocumentTypeError { .. } => 1024,
//         Self::MissingDataContractIdError { .. } => 1025,
//         Self::MissingDocumentTransitionActionError { .. } => 1026,
//         Self::MissingDocumentTransitionTypeError { .. } => 1027,
//         Self::MissingDocumentTypeError => 1028,
//
//         // Identity
//         Self::DuplicatedIdentityPublicKeyBasicError(_) => 1029,
//         Self::DuplicatedIdentityPublicKeyIdBasicError(_) => 1030,
//         Self::IdentityAssetLockProofLockedTransactionMismatchError(_) => 1031,
//         Self::IdentityAssetLockTransactionIsNotFoundError(_) => 1032,
//         Self::IdentityAssetLockTransactionOutPointAlreadyExistsError(_) => 1033,
//         Self::IdentityAssetLockTransactionOutputNotFoundError(_) => 1034,
//         Self::InvalidAssetLockProofCoreChainHeightError(_) => 1035,
//         Self::InvalidAssetLockProofTransactionHeightError(_) => 1036,
//         Self::InvalidAssetLockTransactionOutputReturnSizeError(_) => 1037,
//         Self::InvalidIdentityAssetLockTransactionError(_) => 1038,
//         Self::InvalidIdentityAssetLockTransactionOutputError(_) => 1039,
//         Self::InvalidIdentityPublicKeyDataError(_) => 1040,
//         Self::InvalidInstantAssetLockProofError(_) => 1041,
//         Self::InvalidInstantAssetLockProofSignatureError(_) => 1042,
//         Self::MissingMasterPublicKeyError(_) => 1046,
//         Self::InvalidIdentityPublicKeySecurityLevelError(_) => 1047,
//         Self::InvalidIdentityKeySignatureError { .. } => 1056,
//         Self::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(_) => 1057,
//         Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(_) => 1058,
//         Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(_) => 1059,
//
//         // State Transition
//         Self::InvalidStateTransitionTypeError { .. } => 1043,
//         Self::MissingStateTransitionTypeError => 1044,
//         Self::StateTransitionMaxSizeExceededError { .. } => 1045,
//     };
//
//     Ok(error)
// }
//
// fn create_signature_consensus_error_from_code(code: u32, args: &Vec<Value>) -> Result<SignatureError, ProtocolError> {
//     match code {
//         1000...1999 => BasicError::ProtocolVersionParsingError
//         1001 =>
//             { .. } => 1000,
//         Self::SerializedObjectParsingError { .. } => ,
//         Self::UnsupportedProtocolVersionError(_) => 1002,
//         Self::IncompatibleProtocolVersionError(_) => 1003,
//     }
// }
//
// fn create_fee_consensus_error_from_code(code: u32, args: &Vec<Value>) -> Result<FeeError, ProtocolError> {
//     match code {
//         1000...1999 => BasicError::ProtocolVersionParsingError
//         1001 =>
//             { .. } => 1000,
//         Self::SerializedObjectParsingError { .. } => ,
//         Self::UnsupportedProtocolVersionError(_) => 1002,
//         Self::IncompatibleProtocolVersionError(_) => 1003,
//     }
// }
//
// fn create_state_consensus_error_from_code(code: u32, args: &Vec<Value>) -> Result<StateError, ProtocolError> {
//     match code {
//         1000...1999 => BasicError::ProtocolVersionParsingError
//         1001 =>
//             { .. } => 1000,
//         Self::SerializedObjectParsingError { .. } => ,
//         Self::UnsupportedProtocolVersionError(_) => 1002,
//         Self::IncompatibleProtocolVersionError(_) => 1003,
//     }
// }
//
// fn create_protocol_anyhow_error(args: &Vec<Value>) -> Result<ProtocolError::Error, ProtocolError> {
//     let parsing_error_value = get_index(args, 0)?;
//
//     let parsing_error_string = parsing_error_value.as_text()
//         .ok_or(ProtocolError::ValueError(ValueError::StringDecodingError("parse error must be an array".into())))?;
//
//     Ok(ProtocolError::Error(anyhow!(parsing_error_string)))
// }
//
// fn get_index(args: &Vec<Value>, index: u32) -> Result<&Value, ProtocolError> {
//     args.get(index)
//         .ok_or(ProtocolError::ValueError(ValueError::StructureError(format!("index {} should exist", index))))
// }
