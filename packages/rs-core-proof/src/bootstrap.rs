//! The bootstrap envelope: a proof chain plus Merkle openings of records under
//! the target's roots, as `getquorumproofchain` returns in `bootstrap_hex`.
use crate::chain::{Progress, QuorumProofChain, MAX_PROOF_BYTES};
use crate::merkle::MerklePath;
use crate::state::State;
use crate::wire::{sha256d, Reader, Result};
use crate::Error;

/// Most records one envelope may open, as in Core.
const MAX_RECORDS: u8 = 16;
/// Largest record leaf, as in Core.
const MAX_LEAF_BYTES: usize = 4096;

/// Which target root a record is opened under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    /// A serialized quorum commitment under the quorum root.
    Quorum,
    /// A simplified masternode list entry, without its version, under the
    /// masternode root.
    Masternode,
}

/// A record the envelope opened under the verified target state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedRecord<'a> {
    /// The root it is opened under.
    pub kind: RecordKind,
    /// The leaf's preimage.
    pub leaf: &'a [u8],
}

/// The result of verifying an envelope. Only [`verify`] constructs it.
#[derive(Debug)]
pub struct VerifiedBootstrap<'a> {
    state: State,
    records: Vec<VerifiedRecord<'a>>,
}

impl<'a> VerifiedBootstrap<'a> {
    /// The authenticated target state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// The records, in envelope order.
    pub fn records(&self) -> &[VerifiedRecord<'a>] {
        &self.records
    }
}

/// Verify an envelope from the `trusted` checkpoint, requiring a target at or
/// above `minimum_height`.
///
/// Records are returned only after the chain and every opening verify. The
/// caller still has to check that the records are the ones it asked for and
/// decode their contents.
pub fn verify<'a>(
    bytes: &'a [u8],
    trusted: &State,
    minimum_height: u32,
) -> Result<VerifiedBootstrap<'a>> {
    start(bytes, trusted, minimum_height)?.finish()
}

/// Begin verifying an envelope as [`verify`] does, so the caller can verify
/// its chain's links one at a time with [`Verification::verify_link`] and do
/// other work between them. Each link costs a BLS verification and the X11
/// hashes of its ancestor headers.
///
/// Decodes the envelope and makes every check that comes before the first
/// link; [`Verification::finish`] makes the rest. Verifying links one at a
/// time accepts and rejects exactly what [`verify`] does.
pub fn start<'a>(
    bytes: &'a [u8],
    trusted: &State,
    minimum_height: u32,
) -> Result<Verification<'a>> {
    if bytes.len() > MAX_PROOF_BYTES {
        return Err(Error("bootstrap size limit"));
    }
    let mut records = Reader::new(bytes);
    let chain = QuorumProofChain::decode(records.blob(MAX_PROOF_BYTES)?)?;
    // Decided before any signature work: the height alone makes it useless.
    if chain.target.height < minimum_height {
        return Err(Error::STALE_TARGET);
    }
    let progress = chain.start(trusted)?;
    Ok(Verification {
        chain,
        progress,
        records,
    })
}

/// An envelope part-way through verification. Only [`start`] constructs it.
pub struct Verification<'a> {
    chain: QuorumProofChain,
    progress: Progress,
    /// The envelope after the chain: its records.
    records: Reader<'a>,
}

impl<'a> Verification<'a> {
    /// How many of the chain's links are still to verify.
    pub fn links_left(&self) -> usize {
        self.progress.links_left(&self.chain)
    }

    /// Verify the next link of the chain. An error means the envelope is
    /// invalid, except when no link is left, which is an error too.
    pub fn verify_link(&mut self) -> Result<()> {
        self.chain.verify_link(&mut self.progress)
    }

    /// Verify any links left, then the target and the records, and return
    /// the verified envelope.
    pub fn finish(self) -> Result<VerifiedBootstrap<'a>> {
        let state = self.chain.finish(self.progress)?;
        open_records(self.records, state)
    }
}

/// Open the records after the chain under the verified target `state`.
fn open_records(mut r: Reader<'_>, state: State) -> Result<VerifiedBootstrap<'_>> {
    let count = r.u8()?;
    if !(1..=MAX_RECORDS).contains(&count) {
        return Err(Error("projection count"));
    }
    let mut records = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (kind, root) = match r.u8()? {
            0 => (RecordKind::Quorum, &state.quorum_root),
            1 => (RecordKind::Masternode, &state.masternode_root),
            _ => return Err(Error("record kind")),
        };
        let leaf = r.blob(MAX_LEAF_BYTES)?;
        // A 64-byte leaf could pose as an inner Merkle node.
        if leaf.is_empty() || leaf.len() == 64 {
            return Err(Error("record leaf size"));
        }
        MerklePath::decode(&mut r)?.verify(sha256d(leaf), root)?;
        records.push(VerifiedRecord { kind, leaf });
    }
    r.finish()?;
    Ok(VerifiedBootstrap { state, records })
}
