//! Retired: the validator set types only Platform state structure 0 read.
//!
//! Removed in Platform v5.1.0 by #5323 with the reader in the parent module, to drop the C++
//! `bls-signatures` dependency from the default production node. These types had no decoder of
//! their own, so there is nothing to answer with an error: the parent's decoder refuses the
//! whole record before any of them would be read.
