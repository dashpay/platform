use crate::state_transition::{StateTransition, StateTransitionValueConvert};
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-value-conversion")]
impl StateTransitionValueConvert<'_> for StateTransition {
    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => st.to_object(skip_signature),
            StateTransition::DataContractUpdate(st) => st.to_object(skip_signature),
            StateTransition::Batch(st) => st.to_object(skip_signature),
            StateTransition::IdentityCreate(st) => st.to_object(skip_signature),
            StateTransition::IdentityTopUp(st) => st.to_object(skip_signature),
            StateTransition::IdentityCreditWithdrawal(st) => st.to_object(skip_signature),
            StateTransition::IdentityUpdate(st) => st.to_object(skip_signature),
            StateTransition::IdentityCreditTransfer(st) => st.to_object(skip_signature),
            StateTransition::MasternodeVote(st) => st.to_object(skip_signature),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.to_object(skip_signature),
            StateTransition::IdentityCreateFromAddresses(st) => st.to_object(skip_signature),
            StateTransition::IdentityTopUpFromAddresses(st) => st.to_object(skip_signature),
            StateTransition::AddressFundsTransfer(st) => st.to_object(skip_signature),
            StateTransition::AddressFundingFromAssetLock(st) => st.to_object(skip_signature),
            StateTransition::AddressCreditWithdrawal(st) => st.to_object(skip_signature),
        }
    }

    fn to_canonical_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => st.to_canonical_object(skip_signature),
            StateTransition::DataContractUpdate(st) => st.to_canonical_object(skip_signature),
            StateTransition::Batch(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityCreate(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityTopUp(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityCreditWithdrawal(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityUpdate(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityCreditTransfer(st) => st.to_canonical_object(skip_signature),
            StateTransition::MasternodeVote(st) => st.to_canonical_object(skip_signature),
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.to_canonical_object(skip_signature)
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                st.to_canonical_object(skip_signature)
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                st.to_canonical_object(skip_signature)
            }
            StateTransition::AddressFundsTransfer(st) => st.to_canonical_object(skip_signature),
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.to_canonical_object(skip_signature)
            }
            StateTransition::AddressCreditWithdrawal(st) => st.to_canonical_object(skip_signature),
        }
    }

    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::DataContractUpdate(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::Batch(st) => st.to_canonical_cleaned_object(skip_signature),
            StateTransition::IdentityCreate(st) => st.to_canonical_cleaned_object(skip_signature),
            StateTransition::IdentityTopUp(st) => st.to_canonical_cleaned_object(skip_signature),
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::IdentityUpdate(st) => st.to_canonical_cleaned_object(skip_signature),
            StateTransition::IdentityCreditTransfer(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::MasternodeVote(st) => st.to_canonical_cleaned_object(skip_signature),
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::AddressFundsTransfer(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                st.to_canonical_cleaned_object(skip_signature)
            }
        }
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => st.to_cleaned_object(skip_signature),
            StateTransition::DataContractUpdate(st) => st.to_cleaned_object(skip_signature),
            StateTransition::Batch(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityCreate(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityTopUp(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityCreditWithdrawal(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityUpdate(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityCreditTransfer(st) => st.to_cleaned_object(skip_signature),
            StateTransition::MasternodeVote(st) => st.to_cleaned_object(skip_signature),
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                st.to_cleaned_object(skip_signature)
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                st.to_cleaned_object(skip_signature)
            }
            StateTransition::IdentityTopUpFromAddresses(st) => st.to_cleaned_object(skip_signature),
            StateTransition::AddressFundsTransfer(st) => st.to_cleaned_object(skip_signature),
            StateTransition::AddressFundingFromAssetLock(st) => {
                st.to_cleaned_object(skip_signature)
            }
            StateTransition::AddressCreditWithdrawal(st) => st.to_cleaned_object(skip_signature),
        }
    }

    fn from_object(
        raw_object: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        // Deserialize using existing platform_value deserialization
        // This is already implemented via the derive macros
        platform_value::from_value(raw_object).map_err(ProtocolError::ValueError)
    }

    fn from_value_map(
        raw_value_map: BTreeMap<String, Value>,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(Value::Map(
            raw_value_map
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect(),
        ))
        .map_err(ProtocolError::ValueError)
    }

    fn clean_value(_value: &mut Value) -> Result<(), ProtocolError> {
        // The clean_value logic is variant-specific and should be dispatched
        // However, since we're working with a Value (not a StateTransition instance),
        // we'd need type information to dispatch correctly.
        // For now, this is a no-op as per the default trait implementation.
        Ok(())
    }
}
