//! Verification of Dash Core quorum proof chains (`DASHNC02`, [DIP 175]).
//!
//! An application fixes a trusted checkpoint, a [`State`]: a block's height
//! and hash and the masternode and quorum Merkle roots its coinbase commits
//! to. Core's `getquorumproofchain` RPC proves later state from it. Its
//! `proof_hex` is a [`QuorumProofChain`] ending in a ChainLocked block whose
//! coinbase commits to new roots; its `bootstrap_hex` adds Merkle openings of
//! quorum commitments and masternode entries under those roots, which
//! [`bootstrap::verify`] checks as a whole.
//!
//! The trust model is the checkpoint plus the honesty of the ChainLock quorums
//! the chain passes through. The verifier checks every byte, signature and
//! Merkle path the way Core's verifier does, but it does not replay DKG or
//! re-derive which members signed, and a valid proof does not show that its
//! target is the newest ChainLock: freshness is the caller's policy.
//!
//! Every length is bounded before allocation, and decoding is canonical:
//! re-encoding a decoded chain gives back the same bytes.
//!
//! [DIP 175]: https://github.com/dashpay/dips/pull/175
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bls;
pub mod bootstrap;
mod chain;
mod merkle;
mod state;
mod transaction;
mod wire;

pub use chain::{
    header_hash, Certificate, Link, ProofTransaction, QuorumProofChain, MAX_PROOF_BYTES,
    MAX_PROOF_CERTIFICATES, MAX_PROOF_HEADERS, PROOF_MAGIC,
};
pub use merkle::MerklePath;
pub use state::{from_rpc_hex, rpc_hex, ProofNetwork, State};
pub use transaction::{coinbase_roots, parse_commitment, Commitment, MAX_TRANSACTION_BYTES};
pub use wire::{sha256d, Hash, Result};

/// Why a proof, or a part of one, was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Error {}
