//! Retired: the validator set types only Platform state structure 0 read.
//!
//! Removed in Platform v5.1.0 by #5323 with the reader in the parent module, to drop the C++
//! `bls-signatures` dependency from the default production node. They were read only through the
//! parent's `with_serde` field, so they have no entry point of their own: the parent's decoder
//! refuses the whole record first.
