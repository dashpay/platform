//! Compilation readiness: the state Platform keeps while the evonodes of the network prepare a
//! contract's executable bundle before it can be activated.
//!
//! A [`round::ReadinessRound`] is opened per contract when a bundle is accepted; evonodes send
//! signed readiness reports that Drive stores as [`report_record::ReadinessReportRecord`]
//! entries under the round's count tree; the block event validates the reporters against the
//! block's membership view in pages whose position is a [`scan_cursor::ReadinessScanCursor`],
//! and records the crossing and the activation deadline on the round. The
//! [`payer::ReadinessPayer`] is the party the unused fund is refunded to when the round retires.

pub mod payer;
pub mod report_record;
pub mod round;
pub mod scan_cursor;
