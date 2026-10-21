//! The `DASHNC02` quorum proof chain (Core's `QuorumProofChain`).
//!
//! Starting from a checkpoint's quorum root, the chain opens one ChainLock
//! quorum's commitment, then follows ChainLock certificates: each one signed
//! by the current ChainLock quorum authenticates a block that mined the next
//! ChainLock quorum. The last certificate authenticates a target block whose
//! coinbase commits to new masternode and quorum roots.
use crate::bls;
use crate::merkle::MerklePath;
use crate::state::State;
use crate::transaction::{
    coinbase_roots, mined_commitment, parse_commitment, Commitment, MAX_COMMITMENT_BYTES,
    MAX_TRANSACTION_BYTES,
};
use crate::wire::{sha256d, write_blob, Hash, Reader, Result};
use crate::Error;

/// Eight-byte format identifier: DASH, Network Compact, format 02.
pub const PROOF_MAGIC: &[u8; 8] = b"DASHNC02";
/// Largest encoded proof chain or bootstrap envelope, as in Core.
pub const MAX_PROOF_BYTES: usize = 1024 * 1024;
/// Bound on the number of certificates in a chain, as in Core.
pub const MAX_PROOF_CERTIFICATES: usize = 4096;
/// Bound on the ancestor headers across a whole chain, as in Core.
pub const MAX_PROOF_HEADERS: usize = 4096;

/// A block header's X11 hash, its Dash block hash.
pub fn header_hash(header: &[u8; 80]) -> Hash {
    // The hash's C routine reads exactly 80 bytes whatever the slice length;
    // the array type guarantees that.
    rs_x11_hash::get_x11_hash(header)
}

/// The hash a ChainLock quorum signs for a block at `height`.
fn chainlock_sign_hash(signer: &Commitment, height: u32, block: &Hash) -> Hash {
    let mut request = b"\x05clsig".to_vec();
    request.extend(height.to_le_bytes());
    let mut sign = vec![signer.quorum_type];
    sign.extend(signer.quorum_hash);
    sign.extend(sha256d(&request));
    sign.extend(block);
    sha256d(&sign)
}

/// A ChainLock: a block header at a height, signed by a ChainLock quorum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    /// Height of the block.
    pub height: u32,
    /// The block's 80-byte header.
    pub header: [u8; 80],
    /// The quorum's Basic-scheme signature.
    pub signature: [u8; 96],
}

impl Certificate {
    /// Check the signature by `signer` at a height above `minimum`, and return
    /// the certified block's hash.
    fn verify(&self, signer: &Commitment, minimum: u32) -> Result<Hash> {
        if self.height <= minimum || self.height > i32::MAX as u32 {
            return Err(Error("non-increasing certificate"));
        }
        let hash = header_hash(&self.header);
        let message = chainlock_sign_hash(signer, self.height, &hash);
        bls::verify(&signer.public_key, &self.signature, &message)?;
        Ok(hash)
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.extend(self.height.to_le_bytes());
        out.extend(self.header);
        out.extend(self.signature);
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            height: r.u32()?,
            header: r.array()?,
            signature: r.array()?,
        })
    }
}

/// A complete transaction and its path to a block's transaction Merkle root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofTransaction {
    /// The serialized transaction.
    pub transaction: Vec<u8>,
    /// Its position in the block.
    pub path: MerklePath,
}

impl ProofTransaction {
    fn verify(&self, header: &[u8; 80]) -> Result<()> {
        // A 64-byte transaction could pose as an inner Merkle node.
        if self.transaction.len() == 64 || self.transaction.len() > MAX_TRANSACTION_BYTES {
            return Err(Error("transaction size"));
        }
        let mut root = [0; 32];
        root.copy_from_slice(&header[36..68]);
        self.path.verify(sha256d(&self.transaction), &root)
    }

    fn encode(&self, out: &mut Vec<u8>) -> Result<()> {
        if self.transaction.len() > MAX_TRANSACTION_BYTES {
            return Err(Error("proof transaction limit"));
        }
        write_blob(out, &self.transaction)?;
        self.path.encode(out)
    }

    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            transaction: r.blob(MAX_TRANSACTION_BYTES)?.to_vec(),
            path: MerklePath::decode(r)?,
        })
    }
}

/// One step of the chain: a certificate by the current ChainLock quorum and
/// the transaction that mined the next one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The ChainLock that authenticates this step.
    pub certificate: Certificate,
    /// The next ChainLock quorum's commitment transaction.
    pub mining: ProofTransaction,
    /// Headers from the mining block up to the certified block's parent,
    /// oldest first; empty when the mining block itself is certified.
    pub ancestors: Vec<[u8; 80]>,
}

impl Link {
    /// Check that the mining transaction sits in the certified block or in an
    /// ancestor linked to it, and return the commitment it mines.
    fn mined_commitment(&self) -> Result<Commitment> {
        let ancestors = u32::try_from(self.ancestors.len()).map_err(|_| Error("ancestor limit"))?;
        let height = self
            .certificate
            .height
            .checked_sub(ancestors)
            .filter(|height| *height > 0)
            .ok_or(Error("ancestor budget/height"))?;
        let mut descendant = &self.certificate.header;
        for ancestor in self.ancestors.iter().rev() {
            if descendant[4..36] != header_hash(ancestor) {
                return Err(Error("ancestor continuity"));
            }
            descendant = ancestor;
        }
        if self.mining.path.index == 0 {
            return Err(Error("mining inclusion"));
        }
        self.mining.verify(descendant)?;
        mined_commitment(&self.mining.transaction, height)
    }
}

/// A `DASHNC02` proof chain, as `getquorumproofchain` returns in `proof_hex`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuorumProofChain {
    /// The checkpoint the chain starts from.
    pub anchor: State,
    /// The first ChainLock quorum's serialized commitment.
    pub seed: Vec<u8>,
    /// Its path to the checkpoint's quorum root.
    pub seed_path: MerklePath,
    /// Steps through later ChainLock quorums.
    pub links: Vec<Link>,
    /// The ChainLock of the target block.
    pub target: Certificate,
    /// The target block's coinbase.
    pub coinbase: ProofTransaction,
}

impl QuorumProofChain {
    /// Verify the chain from `trusted` and return the target block's state.
    ///
    /// The chain must start from exactly `trusted`; a relay cannot choose the
    /// checkpoint.
    pub fn verify(&self, trusted: &State) -> Result<State> {
        if &self.anchor != trusted {
            return Err(Error::UNTRUSTED_CHECKPOINT);
        }
        trusted.validate()?;
        if self.links.len() >= MAX_PROOF_CERTIFICATES {
            return Err(Error("certificate limit"));
        }
        let headers: usize = self.links.iter().map(|link| link.ancestors.len()).sum();
        if headers > MAX_PROOF_HEADERS {
            return Err(Error("ancestor budget/height"));
        }
        let chainlock_type = trusted.network.chainlock_quorum_type();
        let ensure_chainlock = |commitment: Commitment| {
            if commitment.quorum_type == chainlock_type {
                Ok(commitment)
            } else {
                Err(Error("unexpected quorum type"))
            }
        };

        let mut signer = ensure_chainlock(parse_commitment(&self.seed)?)?;
        self.seed_path
            .verify(sha256d(&self.seed), &trusted.quorum_root)?;
        let mut height = trusted.height;
        for link in &self.links {
            link.certificate.verify(&signer, height)?;
            let next = ensure_chainlock(link.mined_commitment()?)?;
            if next.quorum_hash == signer.quorum_hash {
                return Err(Error("redundant signer"));
            }
            signer = next;
            height = link.certificate.height;
        }
        let block_hash = self.target.verify(&signer, height)?;
        if self.coinbase.path.index != 0 {
            return Err(Error("coinbase inclusion"));
        }
        self.coinbase.verify(&self.target.header)?;
        let (masternode_root, quorum_root) =
            coinbase_roots(&self.coinbase.transaction, self.target.height)?;
        Ok(State {
            network: trusted.network,
            height: self.target.height,
            block_hash,
            masternode_root,
            quorum_root,
        })
    }

    /// Decode a chain, bounding every allocation by the input's length.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_PROOF_BYTES {
            return Err(Error("proof size limit"));
        }
        let mut r = Reader::new(bytes);
        if r.take(PROOF_MAGIC.len())? != PROOF_MAGIC {
            return Err(Error("proof version"));
        }
        let anchor = State::decode(&mut r)?;
        let seed = r.blob(MAX_COMMITMENT_BYTES)?.to_vec();
        let seed_path = MerklePath::decode(&mut r)?;
        let count = r.u16()? as usize;
        if count >= MAX_PROOF_CERTIFICATES {
            return Err(Error("certificate limit"));
        }
        let mut remaining = MAX_PROOF_HEADERS;
        let mut links = Vec::new();
        for _ in 0..count {
            let certificate = Certificate::decode(&mut r)?;
            let mining = ProofTransaction::decode(&mut r)?;
            let headers = r.u16()? as usize;
            remaining = remaining
                .checked_sub(headers)
                .ok_or(Error("ancestor length/budget"))?;
            // Take every header's bytes before allocating for them.
            let raw = r.take(headers * 80)?;
            let ancestors = raw.as_chunks::<80>().0.to_vec();
            links.push(Link {
                certificate,
                mining,
                ancestors,
            });
        }
        let target = Certificate::decode(&mut r)?;
        let coinbase = ProofTransaction::decode(&mut r)?;
        r.finish()?;
        Ok(Self {
            anchor,
            seed,
            seed_path,
            links,
            target,
            coinbase,
        })
    }

    /// Encode the chain as Core does.
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.links.len() >= MAX_PROOF_CERTIFICATES {
            return Err(Error("certificate limit"));
        }
        if self.seed.len() > MAX_COMMITMENT_BYTES {
            return Err(Error("proof blob length"));
        }
        let mut out = PROOF_MAGIC.to_vec();
        self.anchor.encode(&mut out);
        write_blob(&mut out, &self.seed)?;
        self.seed_path.encode(&mut out)?;
        out.extend((self.links.len() as u16).to_le_bytes());
        let mut remaining = MAX_PROOF_HEADERS;
        for link in &self.links {
            remaining = remaining
                .checked_sub(link.ancestors.len())
                .ok_or(Error("total ancestor limit"))?;
            link.certificate.encode(&mut out);
            link.mining.encode(&mut out)?;
            out.extend((link.ancestors.len() as u16).to_le_bytes());
            for header in &link.ancestors {
                out.extend(header);
            }
            // Checked per link, as in Core, so a huge chain fails early.
            if out.len() > MAX_PROOF_BYTES {
                return Err(Error("proof size limit"));
            }
        }
        self.target.encode(&mut out);
        self.coinbase.encode(&mut out)?;
        if out.len() > MAX_PROOF_BYTES {
            return Err(Error("proof size limit"));
        }
        Ok(out)
    }
}
