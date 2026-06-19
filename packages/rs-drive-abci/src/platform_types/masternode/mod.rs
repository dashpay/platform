use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::masternode::v0::MasternodeV0;
use crate::platform_types::masternode::v1::MasternodeV1;
use bincode::{Decode, Encode};
use dpp::dashcore_rpc::json::MasternodeListItem;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};

mod accessors;
/// Version 0
pub mod v0;
/// Version 1
pub mod v1;

/// `Masternode` represents a masternode on the network.
#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Masternode {
    /// Version 0
    V0(MasternodeV0),
    /// Version 1 — persists the Core 23 platform host (split platform/core host support).
    V1(MasternodeV1),
}

impl TryFromPlatformVersioned<MasternodeListItem> for Masternode {
    type Error = Error;

    // Only the WRITE direction is gated: protocol v12 writes `V1`, earlier versions write
    // `V0` (byte-identical to what they always wrote). The read direction is variant-tag
    // driven (see `From<Masternode> for MasternodeListItem`) so any binary decodes either
    // variant regardless of protocol version — do NOT add a gate there.
    fn try_from_platform_versioned(
        value: MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version.drive_abci.structs.masternode {
            0 => Ok(Self::V0(value.into())),
            1 => Ok(Self::V1(value.into())),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Masternode::try_from_platform_versioned(MasternodeListItem)".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

impl From<Masternode> for MasternodeListItem {
    fn from(value: Masternode) -> Self {
        match value {
            Masternode::V0(v0) => v0.into(),
            Masternode::V1(v1) => v1.into(),
        }
    }
}

/// Format a platform `host:port` entry for a reconstructed Core 23 `addresses` object,
/// bracketing an IPv6 host so the upstream nested-address parser accepts it. A bare
/// `format!("{host}:{port}")` emits an unparsable `2001:db8::1:443` for an IPv6 host, which
/// would make `DMNState::platform_p2p_address()` return `None` after a save/load round-trip
/// and drop the HPMN from validator sets. An IPv4 address or a hostname is emitted as-is; an
/// already-bracketed host is left alone.
pub(super) fn format_platform_address(host: &str, port: u32) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod format_tests {
    use super::format_platform_address;

    #[test]
    fn brackets_only_unbracketed_ipv6() {
        assert_eq!(format_platform_address("192.0.2.2", 443), "192.0.2.2:443");
        assert_eq!(
            format_platform_address("node.example.com", 443),
            "node.example.com:443"
        );
        assert_eq!(
            format_platform_address("2001:db8::1", 36656),
            "[2001:db8::1]:36656"
        );
        // Already bracketed → not double-bracketed.
        assert_eq!(
            format_platform_address("[2001:db8::1]", 36656),
            "[2001:db8::1]:36656"
        );
    }
}
