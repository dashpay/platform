//! The data contracts registered on mainnet and testnet, as Drive stores them.
//!
//! See `README.md` beside this file for where they come from. A test that changes how
//! contracts are parsed, validated, serialized or stored runs over every one of them, so a
//! change that breaks a contract already on a network fails here before it reaches one.

use serde::Deserialize;

/// A network the corpus holds contracts of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Testnet,
}

/// One stored contract.
pub struct CorpusContract {
    pub network: Network,
    /// The contract id, base58.
    pub id: String,
    /// The contract version the network held when the corpus was taken.
    pub version: u32,
    /// The serialized contract, exactly as Drive stores it.
    pub bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct Line {
    id: String,
    version: u32,
    data: String,
}

const MAINNET: &str = include_str!("mainnet.jsonl");
const TESTNET: &str = include_str!("testnet.jsonl");

fn parse(network: Network, lines: &str) -> impl Iterator<Item = CorpusContract> + '_ {
    lines.lines().map(move |line| {
        let Line { id, version, data } =
            serde_json::from_str(line).expect("expected a corpus line to be valid JSON");
        let bytes = dpp::platform_value::string_encoding::decode(
            &data,
            dpp::platform_value::string_encoding::Encoding::Base64,
        )
        .expect("expected a corpus contract to be base64");
        CorpusContract {
            network,
            id,
            version,
            bytes,
        }
    })
}

/// Every contract of the corpus, mainnet first.
pub fn contracts() -> Vec<CorpusContract> {
    parse(Network::Mainnet, MAINNET)
        .chain(parse(Network::Testnet, TESTNET))
        .collect()
}
