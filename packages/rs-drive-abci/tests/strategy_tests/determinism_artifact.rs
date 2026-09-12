//! The determinism artifact: what one recorded replay leaves behind so that a
//! run on another architecture can be compared with it.
//!
//! The artifact has three sections with three different comparison rules,
//! enforced by `.github/scripts/compare-determinism-artifacts.py`:
//!
//! - `consensus` must be byte-identical between two recordings. Every field
//!   in it is something masternodes agree on: application hashes, transition
//!   results and fees, the protocol version each block ran under, identity
//!   balances and the credit totals.
//! - `profile` describes the recording. The protocol versions must match (two
//!   recordings under different profiles are not comparable) and the target
//!   architecture must differ (comparing a machine with itself proves
//!   nothing). CPU features and the engine pin are recorded so a mismatch can
//!   be explained, never compared.
//! - `diagnostic` is recorded and reported only. Timings differ between
//!   machines by construction; raw engine fuel goes here unless a protocol
//!   version makes it consensus-visible, in which case it moves to
//!   `consensus`.
//!
//! Serialisation is canonical: every map is a `BTreeMap`, every hash is
//! lowercase hex, and `serde_json` writes struct fields in declaration order.

use dpp::balances::total_credits_balance::TotalCreditsBalance;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Bump when a field is added, removed or changes meaning. The comparator
/// refuses artifacts whose schema it does not know.
pub const DETERMINISM_ARTIFACT_SCHEMA: u32 = 1;

/// Environment variable naming the directory the artifact is written to.
/// Unset means the test only asserts in-process.
pub const ARTIFACT_DIR_ENV: &str = "PLATFORM_DETERMINISM_ARTIFACT_DIR";

/// One recorded replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismArtifact {
    pub schema: u32,
    pub profile: Profile,
    pub consensus: Consensus,
    pub diagnostic: Diagnostic,
}

/// The machine and protocol profile the recording ran under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub target_arch: String,
    pub target_os: String,
    pub pointer_width: u32,
    pub endian: String,
    /// Runtime-detected CPU features. Recorded, never compared.
    pub cpu_features: Vec<String>,
    /// Protocol version the chain started at. Must match.
    pub protocol_version_start: u32,
    /// Protocol version the chain ended at. Must match.
    pub protocol_version_end: u32,
    /// The contract engine pin: build, code generator settings, memory guard
    /// layout, NaN canonicalisation. `None` until the engine lands in the
    /// tree; the crate that adds it fills this in so a trapping case can be
    /// tied to the settings that produced it.
    pub engine: Option<EnginePin>,
}

/// Target-specific engine settings that a trap or a cost depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnginePin {
    pub name: String,
    pub version: String,
    pub settings: BTreeMap<String, String>,
}

/// Everything two runs must agree on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consensus {
    pub workload_seed: u64,
    /// Height of the last block executed before the platform was closed and
    /// reopened from its persisted state. Zero when the run was continuous.
    pub reopened_at_height: u64,
    pub blocks: Vec<BlockRecord>,
    /// Lowercase hex of the last committed application hash.
    pub final_root_hash: String,
    pub total_credits: TotalCreditsRecord,
    /// Lowercase hex identity id to credits, sorted by id.
    pub identity_balances: BTreeMap<String, u64>,
}

/// One finalized block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRecord {
    pub height: u64,
    pub protocol_version: u64,
    /// Lowercase hex of the application hash this block committed.
    pub app_hash: String,
    pub transitions: Vec<TransitionRecord>,
}

/// One state transition the proposer kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub name: String,
    pub code: u32,
    pub fee: i64,
}

/// The credit totals at the last block, field for field from
/// `TotalCreditsBalance`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalCreditsRecord {
    pub total_credits_in_platform: u64,
    pub total_in_pools: i64,
    pub total_identity_balances: i64,
    pub total_specialized_balances: i64,
    pub total_in_addresses: i64,
    pub total_in_shielded_balances: i64,
}

impl From<&TotalCreditsBalance> for TotalCreditsRecord {
    fn from(balance: &TotalCreditsBalance) -> Self {
        Self {
            total_credits_in_platform: balance.total_credits_in_platform,
            total_in_pools: balance.total_in_pools,
            total_identity_balances: balance.total_identity_balances,
            total_specialized_balances: balance.total_specialized_balances,
            total_in_addresses: balance.total_in_addresses,
            total_in_shielded_balances: balance.total_in_shielded_balances,
        }
    }
}

/// Recorded and reported, never compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub elapsed_ms: u64,
    pub block_count: u64,
    /// Raw engine fuel per trapping case. Stays here while fuel does not
    /// affect consensus costs or limits; a protocol version that makes it
    /// consensus-visible moves the field to `Consensus`.
    pub engine_fuel: Option<BTreeMap<String, u64>>,
}

impl Profile {
    /// The profile of the machine this test binary runs on.
    pub fn for_this_target(protocol_version_start: u32, protocol_version_end: u32) -> Self {
        Self {
            target_arch: std::env::consts::ARCH.to_string(),
            target_os: std::env::consts::OS.to_string(),
            pointer_width: usize::BITS,
            endian: if cfg!(target_endian = "little") {
                "little".to_string()
            } else {
                "big".to_string()
            },
            cpu_features: detected_cpu_features(),
            protocol_version_start,
            protocol_version_end,
            engine: None,
        }
    }
}

/// The CPU features the acceptance asks us to record next to every
/// recording. Only features that change code generation or float behaviour
/// are listed; the list is a diagnostic, not a gate.
#[cfg(target_arch = "x86_64")]
fn detected_cpu_features() -> Vec<String> {
    let mut features = Vec::new();
    macro_rules! record {
        ($($name:tt),* $(,)?) => {
            $(
                if std::arch::is_x86_feature_detected!($name) {
                    features.push($name.to_string());
                }
            )*
        };
    }
    record!(
        "sse3", "ssse3", "sse4.1", "sse4.2", "popcnt", "avx", "avx2", "bmi1", "bmi2", "lzcnt",
        "fma",
    );
    features
}

#[cfg(target_arch = "aarch64")]
fn detected_cpu_features() -> Vec<String> {
    let mut features = Vec::new();
    macro_rules! record {
        ($($name:tt),* $(,)?) => {
            $(
                if std::arch::is_aarch64_feature_detected!($name) {
                    features.push($name.to_string());
                }
            )*
        };
    }
    record!("neon", "lse", "crc", "aes", "sha2", "dotprod");
    features
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn detected_cpu_features() -> Vec<String> {
    Vec::new()
}

impl DeterminismArtifact {
    /// Canonical bytes: fields in declaration order, maps sorted, no
    /// insignificant whitespace differences between two encodings of equal
    /// values.
    pub fn to_canonical_json(&self) -> Vec<u8> {
        let mut bytes =
            serde_json::to_vec_pretty(self).expect("the artifact has no unserialisable field");
        bytes.push(b'\n');
        bytes
    }

    /// File name the workflow expects: one per target architecture.
    pub fn file_name(&self) -> String {
        format!("determinism-{}.json", self.profile.target_arch)
    }

    /// Writes the artifact into the directory named by
    /// `PLATFORM_DETERMINISM_ARTIFACT_DIR`, creating it if needed. Returns the
    /// path written, or `None` when the variable is unset.
    pub fn write_if_requested(&self) -> io::Result<Option<PathBuf>> {
        let Some(dir) = std::env::var_os(ARTIFACT_DIR_ENV) else {
            return Ok(None);
        };
        let dir = Path::new(&dir);
        fs::create_dir_all(dir)?;
        let path = dir.join(self.file_name());
        fs::write(&path, self.to_canonical_json())?;
        Ok(Some(path))
    }
}

/// Lowercase hex, the only byte encoding the artifact uses.
pub fn hex_lower(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DeterminismArtifact {
        DeterminismArtifact {
            schema: DETERMINISM_ARTIFACT_SCHEMA,
            profile: Profile::for_this_target(13, 14),
            consensus: Consensus {
                workload_seed: 7,
                reopened_at_height: 70,
                blocks: vec![
                    BlockRecord {
                        height: 1,
                        protocol_version: 13,
                        app_hash: hex_lower(&[1u8; 32]),
                        transitions: vec![TransitionRecord {
                            name: "IdentityCreate".to_string(),
                            code: 0,
                            fee: 1_000,
                        }],
                    },
                    BlockRecord {
                        height: 2,
                        protocol_version: 14,
                        app_hash: hex_lower(&[2u8; 32]),
                        transitions: vec![],
                    },
                ],
                final_root_hash: hex_lower(&[2u8; 32]),
                total_credits: TotalCreditsRecord {
                    total_credits_in_platform: 10,
                    total_in_pools: 1,
                    total_identity_balances: 9,
                    total_specialized_balances: 0,
                    total_in_addresses: 0,
                    total_in_shielded_balances: 0,
                },
                identity_balances: [(hex_lower(&[9u8; 32]), 9u64)].into_iter().collect(),
            },
            diagnostic: Diagnostic {
                elapsed_ms: 1,
                block_count: 2,
                engine_fuel: None,
            },
        }
    }

    #[test]
    fn should_encode_the_determinism_artifact_canonically() {
        let artifact = sample();
        let first = artifact.to_canonical_json();
        let second = artifact.clone().to_canonical_json();
        assert_eq!(
            first, second,
            "two encodings of one value must be identical"
        );

        let decoded: DeterminismArtifact =
            serde_json::from_slice(&first).expect("the artifact must round-trip");
        assert_eq!(decoded, artifact);

        let mut mutated = artifact.clone();
        mutated.consensus.blocks[1].app_hash = hex_lower(&[3u8; 32]);
        assert_ne!(
            mutated.to_canonical_json(),
            first,
            "a changed application hash must change the bytes"
        );

        let mut mutated = artifact.clone();
        mutated.consensus.blocks[0].transitions[0].fee += 1;
        assert_ne!(
            mutated.to_canonical_json(),
            first,
            "a changed fee must change the bytes"
        );

        let mut mutated = artifact;
        mutated.diagnostic.elapsed_ms += 1;
        assert_ne!(
            mutated.to_canonical_json(),
            first,
            "the diagnostic section is part of the file even though the comparator ignores it"
        );
    }

    #[test]
    fn should_name_the_artifact_after_the_target_architecture() {
        let artifact = sample();
        assert_eq!(
            artifact.file_name(),
            format!("determinism-{}.json", std::env::consts::ARCH)
        );
        assert_eq!(artifact.profile.pointer_width, usize::BITS);
        assert!(artifact.profile.engine.is_none());
    }
}
