#[cfg(test)]
mod tests {
    use crate::*;
    use std::ptr;

    #[test]
    fn test_sdk_initialization() {
        unsafe {
            swift_dash_sdk_init();
        }
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(SwiftDashErrorCode::Success as i32, 0);
        assert_eq!(SwiftDashErrorCode::InvalidParameter as i32, 1);
        assert_eq!(SwiftDashErrorCode::InvalidState as i32, 2);
        assert_eq!(SwiftDashErrorCode::NetworkError as i32, 3);
    }

    #[test]
    fn test_network_enum() {
        assert_eq!(SwiftDashNetwork::Mainnet as i32, 0);
        assert_eq!(SwiftDashNetwork::Testnet as i32, 1);
        assert_eq!(SwiftDashNetwork::Devnet as i32, 2);
        assert_eq!(SwiftDashNetwork::Local as i32, 3);
    }
}