//! Pure transaction decoding — consensus-decode raw Dash transaction bytes
//! into structured inputs/outputs with derived addresses. **No wallet state,
//! no FFI, no `unsafe`.**
//!
//! Wallet persistence stores every transaction's raw bytes but only
//! materializes TXO rows for the wallet's *own* outputs, so callers that need
//! to match arbitrary transactions against external addresses (e.g. the
//! CrowdNode on-chain API: "did a tx pay amount X to address Y?") need to
//! reconstruct per-output `(address, amount)` from the stored bytes. This is
//! that primitive. The C-ABI wrapper lives in `platform-wallet-ffi`; this
//! module owns the logic and its unit tests.
//!
//! - Output `address` comes from [`Address::from_script`]; `None` for
//!   non-standard scripts (OP_RETURN, bare multisig, …).
//! - Input `address` is recovered only for P2PKH spends — the last data push
//!   of the scriptSig parsed as a public key — mirroring what wallet UIs
//!   conventionally show as "sent from". `None` otherwise.

use dashcore::blockdata::script::Instruction;
use dashcore::consensus::deserialize;
use dashcore::{Address, Network, OutPoint, PublicKey, ScriptBuf, Transaction, Txid};

/// A consensus-decoded transaction with addresses rendered for a network.
#[derive(Debug, Clone)]
pub struct DecodedTransaction {
    pub txid: Txid,
    pub inputs: Vec<DecodedInput>,
    pub outputs: Vec<DecodedOutput>,
}

/// One decoded input: the previous outpoint and a best-effort sender address.
#[derive(Debug, Clone)]
pub struct DecodedInput {
    pub previous_output: OutPoint,
    /// Best-effort P2PKH sender address recovered from the scriptSig; `None`
    /// for coinbase / non-P2PKH / unparseable inputs.
    pub address: Option<Address>,
}

/// One decoded output: destination address, value, and raw script.
#[derive(Debug, Clone)]
pub struct DecodedOutput {
    /// Destination address for standard scripts (P2PKH / P2SH); `None` for
    /// non-standard scripts.
    pub address: Option<Address>,
    pub value_duffs: u64,
    pub script_pubkey: ScriptBuf,
}

/// Consensus-decode `tx_bytes` and resolve per-output/-input addresses for
/// `network`. Rejects trailing bytes after a valid transaction (uses
/// [`deserialize`], not a streaming decode), so a valid prefix followed by
/// garbage is an error rather than a silent trim.
pub fn decode_transaction(
    tx_bytes: &[u8],
    network: Network,
) -> Result<DecodedTransaction, dashcore::consensus::encode::Error> {
    let tx: Transaction = deserialize(tx_bytes)?;

    let inputs = tx
        .input
        .iter()
        .map(|txin| {
            let address = if txin.previous_output.is_null() {
                None // coinbase
            } else {
                input_address_from_script_sig(&txin.script_sig, network)
            };
            DecodedInput {
                previous_output: txin.previous_output,
                address,
            }
        })
        .collect();

    let outputs = tx
        .output
        .iter()
        .map(|txout| DecodedOutput {
            address: Address::from_script(&txout.script_pubkey, network).ok(),
            value_duffs: txout.value,
            script_pubkey: txout.script_pubkey.clone(),
        })
        .collect();

    Ok(DecodedTransaction {
        txid: tx.txid(),
        inputs,
        outputs,
    })
}

/// Best-effort P2PKH sender address from a scriptSig: take the last data push,
/// parse it as a public key, hash to a P2PKH address. Returns `None` for
/// coinbase, empty, non-push, or non-P2PKH script sigs.
fn input_address_from_script_sig(script_sig: &ScriptBuf, network: Network) -> Option<Address> {
    let mut last_push: Option<Vec<u8>> = None;
    for instruction in script_sig.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(bytes)) => last_push = Some(bytes.as_bytes().to_vec()),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    let candidate = last_push?;
    if candidate.len() != 33 && candidate.len() != 65 {
        return None;
    }
    let pubkey = PublicKey::from_slice(&candidate).ok()?;
    Some(Address::p2pkh(&pubkey, network))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::consensus::serialize;
    use dashcore::hashes::Hash;
    use dashcore::secp256k1::{Secp256k1, SecretKey};
    use dashcore::{TxIn, TxOut, Witness};

    fn test_pubkey() -> PublicKey {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x42u8; 32]).expect("valid secret key");
        PublicKey::new(sk.public_key(&secp))
    }

    /// One P2PKH input (spending 11..11:3), one P2PKH output of 151072 duffs,
    /// one OP_RETURN output. The 151072 amount mirrors a CrowdNode signUp
    /// signal so the fixture doubles as documentation.
    fn synthetic_tx(network: Network) -> (Transaction, Address) {
        let pubkey = test_pubkey();
        let addr = Address::p2pkh(&pubkey, network);
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice(&[0x30u8; 71])
            .push_key(&pubkey)
            .into_script();
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x11u8; 32]),
                    vout: 3,
                },
                script_sig,
                sequence: 0xffffffff,
                witness: Witness::default(),
            }],
            output: vec![
                TxOut {
                    value: 151_072,
                    script_pubkey: addr.script_pubkey(),
                },
                TxOut {
                    value: 0,
                    script_pubkey: ScriptBuf::new_op_return(&[0xAAu8; 4]),
                },
            ],
            special_transaction_payload: None,
        };
        (tx, addr)
    }

    #[test]
    fn decodes_outputs_with_addresses_and_values() {
        let (tx, addr) = synthetic_tx(Network::Testnet);
        let decoded = decode_transaction(&serialize(&tx), Network::Testnet).unwrap();

        assert_eq!(decoded.txid, tx.txid());
        assert_eq!(decoded.outputs.len(), 2);

        assert_eq!(decoded.outputs[0].address.as_ref(), Some(&addr));
        assert_eq!(decoded.outputs[0].value_duffs, 151_072);
        assert_eq!(decoded.outputs[0].script_pubkey, addr.script_pubkey());
        assert!(
            addr.to_string().starts_with('y'),
            "testnet P2PKH starts with 'y'"
        );

        // OP_RETURN: no address, script still present.
        assert!(decoded.outputs[1].address.is_none());
        assert!(!decoded.outputs[1].script_pubkey.as_bytes().is_empty());
    }

    #[test]
    fn recovers_p2pkh_input_address_from_script_sig() {
        let (tx, addr) = synthetic_tx(Network::Testnet);
        let decoded = decode_transaction(&serialize(&tx), Network::Testnet).unwrap();

        assert_eq!(decoded.inputs.len(), 1);
        assert_eq!(
            decoded.inputs[0].previous_output.txid,
            Txid::from_byte_array([0x11u8; 32])
        );
        assert_eq!(decoded.inputs[0].previous_output.vout, 3);
        assert_eq!(decoded.inputs[0].address.as_ref(), Some(&addr));
    }

    #[test]
    fn network_changes_rendered_addresses() {
        let (tx, testnet_addr) = synthetic_tx(Network::Testnet);
        let decoded = decode_transaction(&serialize(&tx), Network::Mainnet).unwrap();
        let rendered = decoded.outputs[0].address.as_ref().unwrap().to_string();
        assert_ne!(rendered, testnet_addr.to_string());
        assert!(rendered.starts_with('X'), "mainnet P2PKH starts with 'X'");
    }

    #[test]
    fn coinbase_input_has_no_address() {
        let (mut tx, _) = synthetic_tx(Network::Testnet);
        tx.input[0].previous_output = OutPoint::null();
        tx.input[0].script_sig = ScriptBuf::new();
        let decoded = decode_transaction(&serialize(&tx), Network::Testnet).unwrap();
        assert!(decoded.inputs[0].address.is_none());
    }

    #[test]
    fn trailing_garbage_is_an_error() {
        let (tx, _) = synthetic_tx(Network::Testnet);
        let mut bytes = serialize(&tx);
        bytes.extend_from_slice(&[0xDE, 0xAD]);
        assert!(decode_transaction(&bytes, Network::Testnet).is_err());
    }

    #[test]
    fn garbage_bytes_are_an_error() {
        assert!(decode_transaction(&[0xFFu8; 16], Network::Testnet).is_err());
    }
}
