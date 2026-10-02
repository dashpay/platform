mod v0;

use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_version::version::PlatformVersion;
use std::fmt;
pub use v0::ReadinessScanCursorV0;

/// The persisted position of a paged eligibility walk over a round's reports.
///
/// A walk is bound to the membership view it started under (`core_height` and `hpmn_len`); a
/// block whose view differs discards the cursor and restarts from the first report, so a
/// crossing is only ever committed from a walk completed against one coherent view.
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
pub enum ReadinessScanCursor {
    /// Version 0.
    V0(ReadinessScanCursorV0),
}

impl fmt::Display for ReadinessScanCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadinessScanCursor::V0(cursor) => write!(f, "V0({})", cursor),
        }
    }
}

impl ReadinessScanCursor {
    /// Starts a cursor for a walk under the membership view identified by `core_height` with
    /// `hpmn_len` eligible evonodes, in the structure version the platform version selects.
    pub fn new(
        core_height: u32,
        hpmn_len: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .readiness_scan_cursor_default_structure_version
        {
            0 => Ok(ReadinessScanCursorV0 {
                core_height,
                hpmn_len,
                next_pro_tx_hash: None,
                eligible_so_far: 0,
                pruned_so_far: 0,
                examined_so_far: 0,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ReadinessScanCursor::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Whether the cursor was taken under the given membership view.
    pub fn is_bound_to(&self, core_height: u32, hpmn_len: u32) -> bool {
        match self {
            ReadinessScanCursor::V0(cursor) => {
                cursor.core_height == core_height && cursor.hpmn_len == hpmn_len
            }
        }
    }

    /// The core height of the membership view the walk runs under.
    pub fn core_height(&self) -> u32 {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.core_height,
        }
    }

    /// The number of eligible evonodes of the membership view the walk runs under.
    pub fn hpmn_len(&self) -> u32 {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.hpmn_len,
        }
    }

    /// The last report key examined; the next page starts after it. `None` before the first
    /// page.
    pub fn next_pro_tx_hash(&self) -> Option<[u8; 32]> {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.next_pro_tx_hash,
        }
    }

    /// Reports found eligible so far.
    pub fn eligible_so_far(&self) -> u32 {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.eligible_so_far,
        }
    }

    /// Reports pruned so far.
    pub fn pruned_so_far(&self) -> u32 {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.pruned_so_far,
        }
    }

    /// Reports examined so far.
    pub fn examined_so_far(&self) -> u32 {
        match self {
            ReadinessScanCursor::V0(cursor) => cursor.examined_so_far,
        }
    }

    /// Advances the cursor past a page that examined `examined` reports ending at
    /// `last_pro_tx_hash`, of which `eligible` were eligible and `pruned` were pruned.
    ///
    /// Eligible and pruned reports are distinct reports of the page, so a page classifying
    /// more reports than it examined is refused, and pages follow report key order, so a page
    /// ending at or before the previous position is refused; either way the cursor is left
    /// unchanged. Restarting the walk takes a new cursor.
    pub fn advance(
        &mut self,
        last_pro_tx_hash: [u8; 32],
        examined: u32,
        eligible: u32,
        pruned: u32,
    ) -> Result<(), ProtocolError> {
        if u64::from(eligible) + u64::from(pruned) > u64::from(examined) {
            return Err(ProtocolError::CorruptedCodeExecution(
                "a readiness scan page classified more reports than it examined".to_string(),
            ));
        }
        match self {
            ReadinessScanCursor::V0(cursor) => {
                // The next page starts strictly after the position, so a repeated or rewound
                // page would count its reports again.
                if cursor
                    .next_pro_tx_hash
                    .is_some_and(|previous| last_pro_tx_hash <= previous)
                {
                    return Err(ProtocolError::CorruptedCodeExecution(
                        "a readiness scan page must end after the previous page".to_string(),
                    ));
                }
                // Every total is checked before any field moves, so an overflow leaves the
                // position and the counts describing the same progress.
                let examined_so_far = cursor
                    .examined_so_far
                    .checked_add(examined)
                    .ok_or(ProtocolError::Overflow("readiness scan examined count"))?;
                let eligible_so_far = cursor
                    .eligible_so_far
                    .checked_add(eligible)
                    .ok_or(ProtocolError::Overflow("readiness scan eligible count"))?;
                let pruned_so_far = cursor
                    .pruned_so_far
                    .checked_add(pruned)
                    .ok_or(ProtocolError::Overflow("readiness scan pruned count"))?;
                cursor.next_pro_tx_hash = Some(last_pro_tx_hash);
                cursor.examined_so_far = examined_so_far;
                cursor.eligible_so_far = eligible_so_far;
                cursor.pruned_so_far = pruned_so_far;
                Ok(())
            }
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
    fn should_round_trip_a_scan_cursor_through_serialization() {
        let platform_version = PlatformVersion::latest();
        let mut cursor = ReadinessScanCursor::new(1_000, 400, platform_version).expect("cursor");
        cursor.advance([7u8; 32], 512, 500, 12).expect("advance");
        let bytes = cursor.serialize_to_bytes().expect("serialize");
        let restored =
            ReadinessScanCursor::deserialize_from_bytes_trusted(&bytes).expect("deserialize");
        assert_eq!(restored, cursor);
        assert_eq!(
            ReadinessScanCursor::deserialize_from_bytes_untrusted(&bytes)
                .expect("deserialize untrusted"),
            restored
        );
        assert!(restored.is_bound_to(1_000, 400));
        assert!(!restored.is_bound_to(1_001, 400));
        assert!(!restored.is_bound_to(1_000, 399));
        assert_eq!(restored.next_pro_tx_hash(), Some([7u8; 32]));
        assert_eq!(restored.examined_so_far(), 512);
        assert_eq!(restored.eligible_so_far(), 500);
        assert_eq!(restored.pruned_so_far(), 12);
    }

    #[test]
    fn should_report_overflow_when_advancing_past_u32() {
        let platform_version = PlatformVersion::latest();
        let mut cursor = ReadinessScanCursor::new(1, 1, platform_version).expect("cursor");
        cursor.advance([1u8; 32], u32::MAX, 0, 0).expect("advance");
        let before = cursor.clone();
        assert!(matches!(
            cursor.advance([2u8; 32], 1, 0, 0),
            Err(ProtocolError::Overflow(_))
        ));
        assert_eq!(cursor, before);
    }

    #[test]
    fn should_refuse_a_page_classifying_more_reports_than_it_examined() {
        let platform_version = PlatformVersion::latest();
        let mut cursor = ReadinessScanCursor::new(1, 1, platform_version).expect("cursor");
        cursor.advance([1u8; 32], 10, 5, 5).expect("advance");
        let before = cursor.clone();
        for (eligible, pruned) in [(3, 1), (0, 4), (u32::MAX, u32::MAX)] {
            assert!(matches!(
                cursor.advance([2u8; 32], 3, eligible, pruned),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(cursor, before);
        }
        assert_eq!(cursor.next_pro_tx_hash(), Some([1u8; 32]));
        assert_eq!(cursor.examined_so_far(), 10);
        assert_eq!(cursor.eligible_so_far(), 5);
        assert_eq!(cursor.pruned_so_far(), 5);
    }

    #[test]
    fn should_refuse_a_page_that_repeats_or_rewinds_the_position() {
        let platform_version = PlatformVersion::latest();
        let mut cursor = ReadinessScanCursor::new(1, 1, platform_version).expect("cursor");
        cursor.advance([2u8; 32], 1, 1, 0).expect("advance");
        let before = cursor.clone();
        for last_pro_tx_hash in [[2u8; 32], [1u8; 32]] {
            assert!(matches!(
                cursor.advance(last_pro_tx_hash, 1, 1, 0),
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
            assert_eq!(cursor, before);
        }
        cursor.advance([3u8; 32], 1, 1, 0).expect("advance");
        assert_eq!(cursor.next_pro_tx_hash(), Some([3u8; 32]));
        assert_eq!(cursor.eligible_so_far(), 2);
    }
}
