use crate::abci::app::PlatformApplication;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn echo<A, C>(_app: &A, request: proto::RequestEcho) -> Result<proto::ResponseEcho, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    Ok(proto::ResponseEcho {
        message: request.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn echo_returns_same_message() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let app = crate::abci::app::FullAbciApplication::new(&platform.platform);

        let request = proto::RequestEcho {
            message: "hello world".to_string(),
        };

        let response = echo::<_, MockCoreRPCLike>(&app, request).expect("echo should succeed");

        assert_eq!(response.message, "hello world");
    }

    #[test]
    fn echo_returns_empty_message() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let app = crate::abci::app::FullAbciApplication::new(&platform.platform);

        let request = proto::RequestEcho {
            message: String::new(),
        };

        let response = echo::<_, MockCoreRPCLike>(&app, request).expect("echo should succeed");

        assert_eq!(response.message, "");
    }

    #[test]
    fn echo_preserves_special_characters() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let app = crate::abci::app::FullAbciApplication::new(&platform.platform);

        let special_msg = "test\n\ttab\r\nnewline unicode: \u{1F600}";
        let request = proto::RequestEcho {
            message: special_msg.to_string(),
        };

        let response = echo::<_, MockCoreRPCLike>(&app, request).expect("echo should succeed");

        assert_eq!(response.message, special_msg);
    }
}
