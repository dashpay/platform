use bincode::{Decode, DecodeUntrusted, Encode};
use std::fmt;

/// Version 0 of an accepted compilation readiness report.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode, DecodeUntrusted)]
pub struct ReadinessReportRecordV0 {
    /// The platform block height at which the report was accepted. Reports accepted in block
    /// `H` count from the readiness event of block `H + 1`, which runs before that block's
    /// transitions.
    pub accepted_at_height: u64,
    /// The preparation profile the reporting node compiled against.
    pub preparation_profile: u16,
}

impl fmt::Display for ReadinessReportRecordV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReadinessReportRecordV0 {{ accepted_at_height: {}, preparation_profile: {} }}",
            self.accepted_at_height, self.preparation_profile
        )
    }
}
