use crate::block::block_info::BlockInfo;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::fmt;

/// What an identity contender vote poll keeps about one contender: when the identity joined and
/// the id of what made it a contender. Together they order the contenders for the tie-break, and
/// the identity id itself is the key the record sits under.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode, DecodeUntrusted)]
pub struct IdentityContenderInfoV0 {
    /// The block the identity joined the poll in. Its time, then its height, order the
    /// contenders: the earliest wins a tie.
    pub joined_at: BlockInfo,
    /// The id of the document or other object that made the identity a contender, for example
    /// a moderation team's application. It breaks a tie between contenders of the same block.
    pub reference_id: Identifier,
}

impl fmt::Display for IdentityContenderInfoV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IdentityContenderInfoV0 {{ joined_at: {}, reference_id: {} }}",
            self.joined_at, self.reference_id
        )
    }
}

/// The versioned record of one contender of an identity contender vote poll.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    From,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[platform_serialize(unversioned)]
pub enum IdentityContenderInfo {
    /// The first version.
    V0(IdentityContenderInfoV0),
}

impl fmt::Display for IdentityContenderInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityContenderInfo::V0(info) => write!(f, "V0({})", info),
        }
    }
}

impl IdentityContenderInfo {
    /// The record of a contender that joined in `joined_at` through `reference_id`, in the
    /// version the platform version selects.
    pub fn new(
        joined_at: BlockInfo,
        reference_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .identity_contender_info_version
        {
            0 => Ok(IdentityContenderInfoV0 {
                joined_at,
                reference_id,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityContenderInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// The block the contender joined in.
    pub fn joined_at(&self) -> &BlockInfo {
        match self {
            IdentityContenderInfo::V0(info) => &info.joined_at,
        }
    }

    /// The id of what made the identity a contender.
    pub fn reference_id(&self) -> Identifier {
        match self {
            IdentityContenderInfo::V0(info) => info.reference_id,
        }
    }

    /// The order of contenders for the tie-break: the earlier block time first, then the lower
    /// block height, then the lower reference id. The first contender of a poll is the least.
    pub fn tie_break_key(&self) -> (u64, u64, Identifier) {
        let joined_at = self.joined_at();
        (joined_at.time_ms, joined_at.height, self.reference_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::epoch::Epoch;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

    fn block(time_ms: u64, height: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            height,
            core_height: 1,
            epoch: Epoch::default(),
        }
    }

    #[test]
    fn should_round_trip_through_platform_serialization() {
        let info = IdentityContenderInfo::new(
            block(1_000, 7),
            Identifier::new([3; 32]),
            PlatformVersion::latest(),
        )
        .expect("expected the info");
        let bytes = info.serialize_to_bytes().expect("serialize");
        let recovered =
            IdentityContenderInfo::deserialize_from_bytes_untrusted(&bytes).expect("deserialize");
        assert_eq!(info, recovered);
    }

    #[test]
    fn tie_break_key_should_order_by_time_then_height_then_reference_id() {
        let platform_version = PlatformVersion::latest();
        let earlier_time =
            IdentityContenderInfo::new(block(1_000, 9), Identifier::new([9; 32]), platform_version)
                .expect("info");
        let lower_height =
            IdentityContenderInfo::new(block(2_000, 5), Identifier::new([9; 32]), platform_version)
                .expect("info");
        let lower_reference =
            IdentityContenderInfo::new(block(2_000, 6), Identifier::new([1; 32]), platform_version)
                .expect("info");
        let last =
            IdentityContenderInfo::new(block(2_000, 6), Identifier::new([2; 32]), platform_version)
                .expect("info");

        assert!(earlier_time.tie_break_key() < lower_height.tie_break_key());
        assert!(lower_height.tie_break_key() < lower_reference.tie_break_key());
        assert!(lower_reference.tie_break_key() < last.tie_break_key());
    }
}
