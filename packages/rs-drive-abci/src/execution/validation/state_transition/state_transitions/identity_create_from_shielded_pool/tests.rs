//! Integration tests for the `IdentityCreateFromShieldedPool` state transition live in the
//! strategy-test / full-block-pipeline suites (credit conservation, denomination boundaries,
//! `total_fee >= denomination` rejection, prove/verify roundtrip, malleability). This module is a
//! placeholder so the `#[cfg(test)] mod tests;` declaration resolves; unit-level coverage of the
//! structural checks lives in the dpp `validate_structure` tests and the drive converter tests.
