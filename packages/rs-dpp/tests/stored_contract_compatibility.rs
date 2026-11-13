//! The bytes a node stores a contract in at protocol version 15, pinned: format version 2, the
//! contract with its document types compiled (stored generation 0).
//!
//! Drive writes these bytes into state and every later binary reads them back without parsing
//! the schemas again, so once protocol version 15 has shipped, what they are and what they read
//! back as can no longer change. The stored form embeds types it does not own through their own
//! encodings (`IdentityKeyReferenceRequirements`, `DocumentPropertyReferenceTarget`,
//! `SettledDeletionRule`, `StorageKeyRequirements`, `SecurityLevel` and others), and the loader
//! rebuilds each type's index structure with the shared builder, so a change far from
//! `serialized_version` can change either. Every system contract, every corpus contract and the
//! contracts written for this fixture are stored here and compared with
//! `fixtures/stored_contracts/stored.jsonl`; see the `README.md` beside it for what the fixture
//! holds and when it may be regenerated.

#[path = "fixtures/contract_corpus/mod.rs"]
mod contract_corpus;

use contract_corpus::{contracts, Network};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV1Getters, DocumentTypeV2Getters,
};
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::DataContract;
use dpp::platform_value::string_encoding::{self, Encoding};
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::util::hash::hash_single;
use dpp::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::{env, fs};

/// The protocol version that introduces the stored format pinned here.
const PROTOCOL_VERSION: u32 = 15;

/// Set to rewrite the fixture from what the code stores now. See `README.md` beside the
/// fixture: allowed only while protocol version 15 is unreleased.
const UPDATE: &str = "UPDATE_STORED_CONTRACT_FIXTURE";

const FIXTURE: &str = "tests/fixtures/stored_contracts/stored.jsonl";

/// The system contracts protocol version 15 has: every slot but the reserved feature flags and
/// the token market contract, which arrives with the token marketplace in protocol version 16.
const SYSTEM_CONTRACTS: [SystemDataContract; 11] = [
    SystemDataContract::Withdrawals,
    SystemDataContract::MasternodeRewards,
    SystemDataContract::DPNS,
    SystemDataContract::Dashpay,
    SystemDataContract::WalletUtils,
    SystemDataContract::TokenHistory,
    SystemDataContract::KeywordSearch,
    SystemDataContract::DocumentHistory,
    SystemDataContract::AppConnect,
    SystemDataContract::ModerationCharters,
    SystemDataContract::DecisionProposals,
];

/// Contracts written for the fixture, in JSON, for what neither the system contracts nor the
/// corpus exercise.
const WRITTEN: [(&str, &str); 1] = [(
    "settled_deletion_and_action_fees.json",
    include_str!("fixtures/stored_contracts/settled_deletion_and_action_fees.json"),
)];

/// What a mismatch means, appended to every failure.
const CHANGED: &str = "The bytes protocol version 15 stores contracts in (format version 2, \
     stored document type generation 0), or what they read back as, changed. Once protocol \
     version 15 has shipped, nodes hold these bytes in state: such a change needs a new stored \
     generation selected by a new protocol version, not an edit of generation 0, of a type it \
     embeds or of the index structure builder it calls. While protocol version 15 is \
     unreleased, regenerate the fixture deliberately, as \
     packages/rs-dpp/tests/fixtures/stored_contracts/README.md describes.";

/// One fixture contract.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Line {
    /// `system <contract>`, `<network> <base58 id>` for a corpus contract, or
    /// `written <file>` for a contract written for the fixture.
    contract: String,
    /// SHA-256 of the stored bytes, hex.
    sha256: String,
    /// SHA-256, hex, of the index structure of each document type the stored bytes read back
    /// with ([`index_structures_of`]).
    index_structures: String,
    /// The stored bytes, base64, for the contracts kept whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bytes: Option<String>,
}

/// Where a fixture contract comes from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    System,
    Corpus(Network),
    Written,
}

/// A contract the fixture pins, as the code builds it at protocol version 15.
struct Built {
    contract: String,
    source: Source,
    built: DataContract,
    stored: Vec<u8>,
}

/// The parts of a contract whose encoding the contracts kept whole must cover: the types stored
/// generation 0 embeds through their own encodings, and what a contract stores beside its
/// document types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Exercises {
    /// `DocumentPropertyReferenceTarget`: `refersTo`, owner, creator or revealed references.
    References,
    /// `IdentityKeyReferenceRequirements`: a key id property referring to an identity key.
    KeyReferences,
    /// `SettledDeletionRule`.
    SettledDeletion,
    /// `StorageKeyRequirements`: the encryption or decryption key a type requires.
    BoundedKeyRequirements,
    /// `GasFeesPaidBy`, in a type's token costs.
    TokenCosts,
    /// `ActionFeePricing`, in a type's action fees.
    ActionFees,
    /// A contested index's field matches and resolution.
    ContestedIndexes,
    /// Token configurations, stored beside the document types.
    Tokens,
    /// Groups, stored beside the document types.
    Groups,
    /// A private type's storage.
    PrivateTypes,
}

const EXERCISES: [Exercises; 10] = [
    Exercises::References,
    Exercises::KeyReferences,
    Exercises::SettledDeletion,
    Exercises::BoundedKeyRequirements,
    Exercises::TokenCosts,
    Exercises::ActionFees,
    Exercises::ContestedIndexes,
    Exercises::Tokens,
    Exercises::Groups,
    Exercises::PrivateTypes,
];

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::get(PROTOCOL_VERSION).expect("expected protocol version 15")
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(hash_single(bytes))
}

/// Reads stored bytes back the way Drive loads a contract from state.
fn read_back(stored: &[u8], contract: &str) -> DataContract {
    DataContract::versioned_deserialize_trusted(stored, false, platform_version()).unwrap_or_else(
        |error| panic!("{contract}: its stored bytes do not read back: {error}\n\n{CHANGED}"),
    )
}

/// Every system contract of protocol version 15, every corpus contract and every contract
/// written for the fixture, as the code builds it at protocol version 15 and stores it.
fn built_contracts() -> Vec<Built> {
    let platform_version = platform_version();
    let system = SYSTEM_CONTRACTS.into_iter().map(|system_contract| {
        let built =
            load_system_data_contract(system_contract, platform_version).unwrap_or_else(|error| {
                panic!("{system_contract:?} does not load at protocol version 15: {error}")
            });
        (format!("system {system_contract:?}"), Source::System, built)
    });
    let corpus = contracts().into_iter().map(|corpus_contract| {
        let network = match corpus_contract.network {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
        };
        let contract = format!("{network} {}", corpus_contract.id);
        // Loaded as Drive loads a contract stored before protocol version 15, from its schemas
        let built = DataContract::versioned_deserialize_trusted(
            &corpus_contract.bytes,
            false,
            platform_version,
        )
        .unwrap_or_else(|error| panic!("{contract} does not load at protocol version 15: {error}"));
        assert_eq!(built.version(), corpus_contract.version, "{contract}");
        (contract, Source::Corpus(corpus_contract.network), built)
    });
    let written = WRITTEN.into_iter().map(|(file, json)| {
        let json = serde_json::from_str(json).expect("expected a written contract to be JSON");
        let built = DataContract::from_json(json, true, platform_version).unwrap_or_else(|error| {
            panic!("{file} does not parse at protocol version 15: {error}")
        });
        (format!("written {file}"), Source::Written, built)
    });
    system
        .chain(corpus)
        .chain(written)
        .map(|(contract, source, built)| {
            let stored = built
                .serialize_to_storage_bytes(platform_version)
                .unwrap_or_else(|error| panic!("{contract} does not store: {error}"));
            assert_eq!(stored[0], 2, "{contract} is not stored in format version 2");
            Built {
                contract,
                source,
                built,
                stored,
            }
        })
        .collect()
}

/// The SHA-256 of the index structure of each of `contract`'s document types, in name order:
/// the levels the loader rebuilds from the stored indexes, which the stored bytes do not hold.
/// Rendered with `Debug`, which shows every field of every level.
fn index_structures_of(contract: &DataContract) -> String {
    let mut rendered = String::new();
    for (name, document_type) in contract.document_types() {
        writeln!(rendered, "{name}: {:?}", document_type.index_structure())
            .expect("expected to write to a string");
    }
    sha256(rendered.as_bytes())
}

/// What `contract` exercises of [`Exercises`].
fn exercises_of(contract: &DataContract) -> BTreeSet<Exercises> {
    let mut exercises = BTreeSet::new();
    if !contract.tokens().is_empty() {
        exercises.insert(Exercises::Tokens);
    }
    if !contract.groups().is_empty() {
        exercises.insert(Exercises::Groups);
    }
    for document_type in contract.document_types().values() {
        for property in document_type.flattened_properties().values() {
            match property.property_type {
                DocumentPropertyType::IdentifierWithReference(_) => {
                    exercises.insert(Exercises::References);
                }
                DocumentPropertyType::KeyIdWithReference(_) => {
                    exercises.insert(Exercises::KeyReferences);
                }
                _ => {}
            }
            if property.revealed_reference.is_some() {
                exercises.insert(Exercises::References);
            }
        }
        if document_type.owner_reference().is_some() || document_type.creator_reference().is_some()
        {
            exercises.insert(Exercises::References);
        }
        if document_type.moderator_settled_deletion().is_some() {
            exercises.insert(Exercises::SettledDeletion);
        }
        if document_type
            .requires_identity_encryption_bounded_key()
            .is_some()
            || document_type
                .requires_identity_decryption_bounded_key()
                .is_some()
        {
            exercises.insert(Exercises::BoundedKeyRequirements);
        }
        if !document_type.all_document_token_costs().is_empty() {
            exercises.insert(Exercises::TokenCosts);
        }
        if document_type.action_fees().is_some() {
            exercises.insert(Exercises::ActionFees);
        }
        if document_type
            .indexes()
            .values()
            .any(|index| index.contested_index.is_some())
        {
            exercises.insert(Exercises::ContestedIndexes);
        }
        if document_type.private_document().is_some() {
            exercises.insert(Exercises::PrivateTypes);
        }
    }
    exercises
}

/// The fixture as the code builds it now. Every system contract, mainnet contract and written
/// contract is kept whole; of testnet's, for each of [`EXERCISES`], the one stored in the fewest
/// bytes that exercises it.
fn fixture_now(built: &[Built]) -> Vec<Line> {
    let testnet = Source::Corpus(Network::Testnet);
    let mut kept_whole: BTreeSet<&str> = built
        .iter()
        .filter(|contract| contract.source != testnet)
        .map(|contract| contract.contract.as_str())
        .collect();
    let testnet: Vec<(&Built, BTreeSet<Exercises>)> = built
        .iter()
        .filter(|contract| contract.source == testnet)
        .map(|contract| (contract, exercises_of(&contract.built)))
        .collect();
    for exercised in EXERCISES {
        if let Some((smallest, _)) = testnet
            .iter()
            .filter(|(_, exercises)| exercises.contains(&exercised))
            .min_by_key(|(contract, _)| contract.stored.len())
        {
            kept_whole.insert(&smallest.contract);
        }
    }
    built
        .iter()
        .map(|contract| Line {
            contract: contract.contract.clone(),
            sha256: sha256(&contract.stored),
            index_structures: index_structures_of(&read_back(&contract.stored, &contract.contract)),
            bytes: kept_whole
                .contains(contract.contract.as_str())
                .then(|| string_encoding::encode(&contract.stored, Encoding::Base64)),
        })
        .collect()
}

/// Where `a` and `b` first differ, if they do.
fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter()
        .zip(b)
        .position(|(a, b)| a != b)
        .or_else(|| (a.len() != b.len()).then_some(a.len().min(b.len())))
}

fn committed_fixture() -> Vec<Line> {
    fs::read_to_string(fixture_path())
        .expect("expected the stored contract fixture")
        .lines()
        .map(|line| serde_json::from_str(line).expect("expected a fixture line to be valid JSON"))
        .collect()
}

fn committed_bytes(line: &Line) -> Option<Vec<u8>> {
    line.bytes.as_ref().map(|bytes| {
        string_encoding::decode(bytes, Encoding::Base64)
            .expect("expected the fixture's bytes to be base64")
    })
}

/// The code stores every fixture contract at protocol version 15 in exactly the committed
/// bytes, and those bytes read back with the committed index structures. With
/// `UPDATE_STORED_CONTRACT_FIXTURE` set, rewrites the fixture instead; see `README.md` beside it
/// for when that is allowed.
#[test]
fn should_store_every_fixture_contract_at_protocol_version_15_in_the_committed_bytes() {
    let built = built_contracts();
    let now = fixture_now(&built);

    if env::var_os(UPDATE).is_some() {
        let mut jsonl = String::new();
        for line in &now {
            jsonl.push_str(&serde_json::to_string(line).expect("expected a line to serialize"));
            jsonl.push('\n');
        }
        fs::write(fixture_path(), jsonl).expect("expected to write the fixture");
        return;
    }

    let committed: BTreeMap<String, Line> = committed_fixture()
        .into_iter()
        .map(|line| (line.contract.clone(), line))
        .collect();
    let contracts_now: BTreeSet<&str> = now.iter().map(|line| line.contract.as_str()).collect();
    let committed_contracts: BTreeSet<&str> = committed.keys().map(String::as_str).collect();
    assert_eq!(
        contracts_now, committed_contracts,
        "the fixture holds other contracts than protocol version 15's system contracts, the \
         corpus and the contracts written for it"
    );

    for (line, contract) in now.iter().zip(&built) {
        let committed_line = &committed[&line.contract];
        if let Some(committed_bytes) = committed_bytes(committed_line) {
            if let Some(offset) = first_difference(&committed_bytes, &contract.stored) {
                panic!(
                    "{}: stored in other bytes than the committed ones, from byte {offset} \
                     ({} bytes now, {} committed)\n\n{CHANGED}",
                    line.contract,
                    contract.stored.len(),
                    committed_bytes.len()
                );
            }
        }
        assert!(
            line.sha256 == committed_line.sha256,
            "{}: stored in other bytes than the committed ones ({} bytes now)\n\n{CHANGED}",
            line.contract,
            contract.stored.len()
        );
        assert!(
            line.index_structures == committed_line.index_structures,
            "{}: its stored bytes read back with other index structures than the committed \
             ones\n\n{CHANGED}",
            line.contract
        );
        assert!(
            line.bytes.is_some() == committed_line.bytes.is_some(),
            "{}: kept whole in the fixture: {}, as the code selects the contracts kept whole: \
             {}\n\n{CHANGED}",
            line.contract,
            committed_line.bytes.is_some(),
            line.bytes.is_some()
        );
    }
}

/// The committed bytes of each contract kept whole read back, with Drive's trusted decoder, as
/// the contract the code builds from its schemas at protocol version 15, and store again as the
/// same bytes. Unlike the comparison of what the code stores, this reads bytes the code did not
/// write: it fails when bytes already in state stop reading back, or read back as another
/// contract.
#[test]
fn should_read_back_the_committed_bytes_as_the_contract_built_from_its_schemas() {
    let platform_version = platform_version();
    let built: BTreeMap<String, Built> = built_contracts()
        .into_iter()
        .map(|contract| (contract.contract.clone(), contract))
        .collect();
    let mut read = 0;
    for line in committed_fixture() {
        let Some(committed_bytes) = committed_bytes(&line) else {
            continue;
        };
        let contract = &line.contract;
        assert_eq!(
            sha256(&committed_bytes),
            line.sha256,
            "{contract}: the fixture's bytes are not the ones its hash names"
        );
        let built = &built
            .get(contract)
            .unwrap_or_else(|| panic!("{contract} is no longer a fixture contract"))
            .built;
        let read_back = read_back(&committed_bytes, contract);
        assert!(
            read_back == *built,
            "{contract}: its committed bytes read back as another contract than its schemas \
             build\n\n{CHANGED}"
        );
        // What document type equality leaves out: the typed validation schema, which every
        // document type of a contract kept whole has
        for name in built.document_types().keys() {
            let validation_schema_of = |of: &DataContract| {
                of.document_type_validation_schema(name, platform_version)
                    .unwrap_or_else(|error| {
                        panic!(
                            "{contract}: document type {name} has no validation schema: \
                             {error}\n\n{CHANGED}"
                        )
                    })
            };
            assert!(
                validation_schema_of(&read_back) == validation_schema_of(built),
                "{contract}: document type {name} reads back with another validation schema \
                 than its schema builds\n\n{CHANGED}"
            );
        }
        assert!(
            index_structures_of(&read_back) == line.index_structures,
            "{contract}: its committed bytes read back with other index structures than the \
             committed ones\n\n{CHANGED}"
        );
        let stored_again = read_back
            .serialize_to_storage_bytes(platform_version)
            .unwrap_or_else(|error| panic!("{contract} does not store again: {error}"));
        assert!(
            stored_again == committed_bytes,
            "{contract}: its committed bytes, read back and stored again, are other bytes\n\n\
             {CHANGED}"
        );
        read += 1;
    }
    assert!(read > 0, "expected the fixture to keep some contract whole");
}

/// Some contract kept whole exercises each part [`Exercises`] lists, and one from the corpus does
/// wherever a corpus contract does, so a change to how any of them is encoded or read back meets
/// bytes stored before it.
#[test]
fn should_keep_whole_a_contract_exercising_each_embedded_type() {
    let built: BTreeMap<String, Built> = built_contracts()
        .into_iter()
        .map(|contract| (contract.contract.clone(), contract))
        .collect();
    let mut kept_whole = BTreeSet::new();
    let mut corpus_kept_whole = BTreeSet::new();
    for line in committed_fixture() {
        if line.bytes.is_none() {
            continue;
        }
        let contract = &built[&line.contract];
        let exercises = exercises_of(&contract.built);
        let from_corpus = matches!(contract.source, Source::Corpus(_));
        if from_corpus {
            corpus_kept_whole.extend(exercises.iter().copied());
        }
        kept_whole.extend(exercises);
    }
    let mut corpus_exercises = BTreeSet::new();
    for contract in built
        .values()
        .filter(|contract| matches!(contract.source, Source::Corpus(_)))
    {
        corpus_exercises.extend(exercises_of(&contract.built));
    }
    for exercised in EXERCISES {
        assert!(
            kept_whole.contains(&exercised),
            "no contract kept whole exercises {exercised:?}"
        );
        assert!(
            !corpus_exercises.contains(&exercised) || corpus_kept_whole.contains(&exercised),
            "a corpus contract exercises {exercised:?}, but none kept whole does"
        );
    }
}
