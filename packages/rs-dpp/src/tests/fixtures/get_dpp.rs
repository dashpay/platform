use crate::{mocks::DashPlatformProtocol, state_repository::MockStateRepositoryLike};

// TODO creation of DPP object for testing needs to be improved
pub fn get_dpp() -> DashPlatformProtocol<MockStateRepositoryLike> {
    DashPlatformProtocol {
        state_repository: MockStateRepositoryLike::new(),
    }
}
