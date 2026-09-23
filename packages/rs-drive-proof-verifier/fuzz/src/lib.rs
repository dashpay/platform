//! Fixture shared by the `FromProof` fuzz targets.
//!
//! Every target feeds the fuzzer's bytes to the verifier as the GroveDB proof
//! of a DAPI response whose Tenderdash envelope (quorum, signature, block id,
//! metadata) is the one recorded in the proof-vector corpus
//! (`../tests/vectors`). All corpus proofs commit to the same root hash, which
//! the fixture quorum signed, so a mutated proof that rebuilds that root passes
//! the signature check and reaches the code that runs after it.
//!
//! That gives every target a soundness oracle on top of crash detection: for a
//! fixed query and a fixed signed root, whatever the verifier accepts must be
//! exactly what the recorded proof of that root yields.
//!
//! Every input is verified under each of [`PLATFORM_VERSIONS`], because the
//! protocol version decides which GroveDB proof envelopes reach GroveDB.

use std::fmt::Debug;
use std::sync::{Arc, LazyLock};

use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::v13::PLATFORM_V13;
use dpp::version::{PlatformVersion, LATEST_PLATFORM_VERSION};
use drive_proof_verifier::{ContextProvider, ContextProviderError, Error, FromProof};

/// The network the fixture chain id (`dash-testnet-51`) belongs to.
const NETWORK: Network = Network::Testnet;

/// The identity every identity fixture proves.
pub const IDENTITY_ID: [u8; 32] = [0x77; 32];

/// GroveDB proofs of the proof-vector corpus, by case name.
pub const IDENTITY_BALANCE_PROOF: &str =
    include_str!("../../tests/vectors/identity-balance/proof.hex");
pub const IDENTITY_CONTRACT_NONCE_PROOF: &str =
    include_str!("../../tests/vectors/identity-contract-nonce/proof.hex");
pub const IDENTITY_KEYS_PROOF: &str = include_str!("../../tests/vectors/identity-keys/proof.hex");
pub const CONTESTED_ACTIVE_PROOF: &str =
    include_str!("../../tests/vectors/contested-vote-state-active/proof.hex");
pub const CONTESTED_FINISHED_PROOF: &str =
    include_str!("../../tests/vectors/contested-vote-state-finished/proof.hex");
pub const CONTESTED_ABSENT_PROOF: &str =
    include_str!("../../tests/vectors/contested-vote-state-absent/proof.hex");

const SIGNATURE: &str = include_str!("../../tests/vectors/quorum-sig-valid/signature.hex");
const QUORUM_PUBLIC_KEY: &str =
    include_str!("../../tests/vectors/quorum-sig-valid/quorum_pubkey.hex");
const QUORUM_HASH: &str = "01080f161d242b323940474e555c636a71787f868d949ba2a9b0b7bec5ccd3da";
const BLOCK_ID_HASH: &str = "090c0f1215181b1e2124272a2d303336393c3f4245484b4e5154575a5d606366";
const QUORUM_TYPE: u32 = 106;

/// The platform versions every target verifies with: 13, the version SDK
/// clients on mainnet and testnet start at, which still accepts legacy V0
/// GroveDB proof envelopes, and the latest, which rejects them before GroveDB
/// decodes them.
pub const PLATFORM_VERSIONS: [&PlatformVersion; 2] = [&PLATFORM_V13, LATEST_PLATFORM_VERSION];

/// The DPNS contract, the only contract the fixture proofs reference.
pub static DPNS_CONTRACT: LazyLock<Arc<DataContract>> = LazyLock::new(|| {
    Arc::new(
        load_system_data_contract(SystemDataContract::DPNS, LATEST_PLATFORM_VERSION)
            .expect("load DPNS contract"),
    )
});

/// Decodes a corpus hex blob.
pub fn decode(hex: &str) -> Vec<u8> {
    hex::decode(hex.trim()).expect("corpus hex blob must decode")
}

/// The block metadata the fixture quorum signed.
pub fn metadata() -> ResponseMetadata {
    ResponseMetadata {
        height: 123456,
        core_chain_locked_height: 2000000,
        epoch: 0,
        time_ms: 1700000000000,
        protocol_version: 12,
        chain_id: "dash-testnet-51".to_string(),
    }
}

/// The signed Tenderdash envelope around `grovedb_proof`.
pub fn proof(grovedb_proof: &[u8]) -> Proof {
    Proof {
        grovedb_proof: grovedb_proof.to_vec(),
        quorum_hash: decode(QUORUM_HASH),
        signature: decode(SIGNATURE),
        round: 0,
        block_id_hash: decode(BLOCK_ID_HASH),
        quorum_type: QUORUM_TYPE,
    }
}

/// Verifies `response` to `request` under `platform_version` against the
/// fixture context.
pub fn verify<R, T: FromProof<R>>(
    platform_version: &PlatformVersion,
    request: impl Into<T::Request>,
    response: impl Into<T::Response>,
) -> Result<Option<T>, Error> {
    T::maybe_from_proof(
        request,
        response,
        NETWORK,
        platform_version,
        &FixtureContextProvider,
    )
}

/// What a recorded proof yields: `verify_with` must accept it under each of
/// [`PLATFORM_VERSIONS`] and return the same result under both.
pub fn recorded<T: PartialEq + Debug>(
    verify_with: impl Fn(&PlatformVersion) -> Result<Option<T>, Error>,
) -> Option<T> {
    let [legacy, latest] = PLATFORM_VERSIONS.map(|platform_version| {
        verify_with(platform_version).unwrap_or_else(|error| {
            panic!(
                "a recorded proof must verify under protocol version {}: {error}",
                platform_version.protocol_version
            )
        })
    });
    assert_eq!(
        legacy, latest,
        "a recorded proof must verify to the same result under every protocol version"
    );
    latest
}

/// Panics when the verifier accepted a result other than `recorded`, the one
/// the recorded proof of the same signed root yields for the same query.
pub fn assert_consistent<T: PartialEq + Debug>(
    verified: Result<Option<T>, Error>,
    recorded: &Option<T>,
) {
    if let Ok(verified) = verified {
        assert_eq!(
            &verified, recorded,
            "a proof of the signed root verified to a result the recorded proof does not yield"
        );
    }
}

/// Serves the fixture quorum key for any quorum and the DPNS contract.
struct FixtureContextProvider;

impl ContextProvider for FixtureContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == DPNS_CONTRACT.id()).then(|| Arc::clone(&DPNS_CONTRACT)))
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(decode(QUORUM_PUBLIC_KEY)
            .try_into()
            .expect("quorum public key must be 48 bytes"))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}
