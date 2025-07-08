use dpp::ProtocolError;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;

mod v0;

pub fn data_trigger_bindings_list(
    platform_version: &PlatformVersion,
) -> Result<Vec<DataTriggerBinding>, ProtocolError> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .data_triggers
        .bindings
    {
        0 => Ok(v0::data_trigger_bindings_list_v0()?
            .into_iter()
            .map(|binding| binding.into())
            .collect()),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "data_trigger_bindings".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
