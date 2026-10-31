use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::list::v2::data_trigger_bindings_list_v2;

use dpp::errors::ProtocolError;
use dpp::system_data_contracts::dashpay_contract;

/// Retrieves a list of data triggers binding with matching params.
///
/// v3 (PROTOCOL_VERSION_15) is v2 without the DashPay bindings. The DashPay
/// contract v3 that protocol version selects declares what both triggers checked:
/// `contactRequest.toUserId` refers to an identity (`refersTo`) other than the
/// writer (`distinctFrom`), and the `profile` rules `corePaymentAddressType` and
/// `platformPaymentAddressType` hold each payment address's type byte to 0x00 or
/// 0x01. Every other binding is v2's.
///
/// # Returns
///
/// A `Vec<DataTriggerBinding>` containing all known data triggers.
///
/// # Errors
///
/// Returns a `ProtocolError` if there was an error.
#[inline(always)]
pub(super) fn data_trigger_bindings_list_v3() -> Result<Vec<DataTriggerBindingV0>, ProtocolError> {
    Ok(data_trigger_bindings_list_v2()?
        .into_iter()
        .filter(|binding| binding.data_contract_id != dashpay_contract::ID)
        .collect())
}
