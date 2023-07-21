use dpp::ProtocolError;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::documents_batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;

mod v0;

// TODO: This function will be updated frequently
//  But we can't separate them because we depends on DataTriggerBinding params
pub fn data_trigger_bindings_list(
    platform_version: &PlatformVersion,
) -> Result<Vec<DataTriggerBinding>, ProtocolError> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .documents_batch_state_transition
        .data_triggers
        .bindings
    {
        0 => v0::data_trigger_bindings_list_v0(),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "data_trigger_bindings".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
