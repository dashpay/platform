mod v0;

use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_version::version::PlatformVersion;
use std::fmt;
pub use v0::ReadinessReportRecordV0;

/// One accepted compilation readiness report, stored under the round's count tree keyed by the
/// reporting evonode's pro tx hash. The count tree's own count is the raw distinct report
/// count; the record carries what the block event needs to judge the report later.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    From,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
)]
#[platform_serialize(unversioned)]
pub enum ReadinessReportRecord {
    /// Version 0.
    V0(ReadinessReportRecordV0),
}

impl fmt::Display for ReadinessReportRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadinessReportRecord::V0(record) => write!(f, "V0({})", record),
        }
    }
}

impl ReadinessReportRecord {
    /// Builds a record for a report accepted at `accepted_at_height` against
    /// `preparation_profile`, in the structure version the platform version selects.
    pub fn new(
        accepted_at_height: u64,
        preparation_profile: u16,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .readiness_report_record_default_structure_version
        {
            0 => Ok(ReadinessReportRecordV0 {
                accepted_at_height,
                preparation_profile,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ReadinessReportRecord::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// The platform block height at which the report was accepted.
    pub fn accepted_at_height(&self) -> u64 {
        match self {
            ReadinessReportRecord::V0(record) => record.accepted_at_height,
        }
    }

    /// The preparation profile the reporting node compiled against.
    pub fn preparation_profile(&self) -> u16 {
        match self {
            ReadinessReportRecord::V0(record) => record.preparation_profile,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{
        PlatformDeserializableTrusted, PlatformDeserializableUntrusted, PlatformSerializable,
    };

    #[test]
    fn should_round_trip_a_report_record_through_serialization() {
        let platform_version = PlatformVersion::latest();
        let record = ReadinessReportRecord::new(42, 3, platform_version).expect("record");
        let bytes = record.serialize_to_bytes().expect("serialize");
        let restored =
            ReadinessReportRecord::deserialize_from_bytes_trusted(&bytes).expect("deserialize");
        assert_eq!(restored, record);
        assert_eq!(
            ReadinessReportRecord::deserialize_from_bytes_untrusted(&bytes)
                .expect("deserialize untrusted"),
            restored
        );
        assert_eq!(restored.accepted_at_height(), 42);
        assert_eq!(restored.preparation_profile(), 3);
    }

    #[test]
    fn should_reject_an_unknown_structure_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .voting_versions
            .readiness_report_record_default_structure_version = 7;
        let result = ReadinessReportRecord::new(1, 1, &platform_version);
        assert!(matches!(
            result,
            Err(ProtocolError::UnknownVersionMismatch { received: 7, .. })
        ));
    }
}
