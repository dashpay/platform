//! Re-export GroveDB's bincode for dependencies that still request upstream 2.0.1.
//!
//! The workspace patch unifies ordinary encoding/decoding trait identities with
//! Platform and GroveDB. It does not opt upstream types into untrusted decoding.
//! Applications consuming Platform through Git/path dependencies must carry the
//! same root-level patch until those dependencies adopt grovedb-bincode themselves.
#![no_std]

pub use bincode_fork::*;
