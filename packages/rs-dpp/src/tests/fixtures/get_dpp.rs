use crate::{
    dash_platform_protocol::DPPOptions, state_repository::MockStateRepositoryLike,
    DashPlatformProtocol,
};

// TODO creation of DPP object for testing needs to be improved
pub fn get_dpp() -> DashPlatformProtocol<MockStateRepositoryLike> {
    DashPlatformProtocol::new(
        DPPOptions {
            current_protocol_version: None,
        },
        MockStateRepositoryLike::new(),
    )
    .unwrap()
}
