use crate::state_transition::{
    JsonStateTransitionSerializationOptions, StateTransition, StateTransitionJsonConvert,
};
use crate::ProtocolError;
use serde_json::Value as JsonValue;

#[cfg(feature = "state-transition-json-conversion")]
impl StateTransitionJsonConvert<'_> for StateTransition {
    fn to_json(
        &self,
        options: JsonStateTransitionSerializationOptions,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            StateTransition::DataContractCreate(st) => st.to_json(options),
            StateTransition::DataContractUpdate(st) => st.to_json(options),
            StateTransition::Batch(st) => st.to_json(options),
            StateTransition::IdentityCreate(st) => st.to_json(options),
            StateTransition::IdentityTopUp(st) => st.to_json(options),
            StateTransition::IdentityCreditWithdrawal(st) => st.to_json(options),
            StateTransition::IdentityUpdate(st) => st.to_json(options),
            StateTransition::IdentityCreditTransfer(st) => st.to_json(options),
            StateTransition::MasternodeVote(st) => st.to_json(options),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.to_json(options),
            StateTransition::IdentityCreateFromAddresses(st) => st.to_json(options),
            StateTransition::IdentityTopUpFromAddresses(st) => st.to_json(options),
            StateTransition::AddressFundsTransfer(st) => st.to_json(options),
            StateTransition::AddressFundingFromAssetLock(st) => st.to_json(options),
            StateTransition::AddressCreditWithdrawal(st) => st.to_json(options),
        }
    }
}
