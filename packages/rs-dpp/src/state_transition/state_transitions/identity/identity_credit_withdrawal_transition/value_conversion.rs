use std::collections::BTreeMap;

use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use platform_value::{BinaryData, Bytes32, Error, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::state_transitions::identity_credit_withdrawal_transition::fields::*;
use crate::state_transition::StateTransitionValueConvert;
use bincode::{config, Decode, Encode};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;

impl StateTransitionValueConvert for IdentityCreditWithdrawalTransition {
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                let mut value = transition.to_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                let mut value = transition.to_canonical_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                let mut value = transition.to_canonical_cleaned_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                let mut value = transition.to_cleaned_object(skip_signature)?;
                value.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))?;
                Ok(value)
            }
        }
    }

    fn from_object(
        mut raw_object: Value,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        let version: u8 = raw_object
            .remove_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;
        match version {
            0 => Ok(IdentityCreditWithdrawalTransitionV0::from_object(raw_object)?.into()),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditWithdrawalTransition version {n}"
            ))),
        }
    }

    fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        let version: u8 = raw_data_contract_create_transition
            .remove_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => Ok(IdentityCreditWithdrawalTransitionV0::from_value_map(
                raw_data_contract_create_transition,
            )?
            .into()),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditWithdrawalTransition version {n}"
            ))),
        }
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        let version: u8 = value
            .get_integer(STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => IdentityCreditWithdrawalTransitionV0::clean_value(value),
            n => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreditWithdrawalTransition version {n}"
            ))),
        }
    }
}
