//! Consensus-decodes raw transaction bytes into the outpoints they spend and
//! the addresses and amounts they pay.

use dashcore::blockdata::script::Instruction;
use dashcore::consensus::{deserialize, encode};
use dashcore::{Address, Network, OutPoint, PublicKey, ScriptBuf, Transaction, Txid};

/// A transaction decoded by [`decode_transaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTransaction {
    pub txid: Txid,
    pub inputs: Vec<DecodedTxInput>,
    pub outputs: Vec<DecodedTxOutput>,
}

/// One decoded transaction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTxInput {
    pub previous_output: OutPoint,
    /// The sender address a P2PKH-shaped scriptSig claims, or `None` for a
    /// coinbase, non-P2PKH or unparsable input. The spender controls every push
    /// and nothing verifies the signature against the spent output, so use it
    /// for display or as a matching hint only, never for authentication. The
    /// per-output `(address, value)` pair is the attacker-resistant primitive.
    pub sender_address: Option<Address>,
}

/// One decoded transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTxOutput {
    /// Destination for a standard script (P2PKH, P2SH), or `None` otherwise.
    pub address: Option<Address>,
    pub value: u64,
    pub script_pubkey: ScriptBuf,
}

/// Consensus-decodes `tx_bytes`. `network` only selects how addresses render.
///
/// Bytes left over after a valid transaction are an error, not a silent trim.
pub fn decode_transaction(
    tx_bytes: &[u8],
    network: Network,
) -> Result<DecodedTransaction, encode::Error> {
    let tx: Transaction = deserialize(tx_bytes)?;
    let txid = tx.txid();

    let inputs = tx
        .input
        .into_iter()
        .map(|txin| DecodedTxInput {
            sender_address: if txin.previous_output.is_null() {
                None
            } else {
                input_address_from_script_sig(&txin.script_sig, network)
            },
            previous_output: txin.previous_output,
        })
        .collect();

    let outputs = tx
        .output
        .into_iter()
        .map(|txout| DecodedTxOutput {
            address: Address::from_script(&txout.script_pubkey, network).ok(),
            value: txout.value,
            script_pubkey: txout.script_pubkey,
        })
        .collect();

    Ok(DecodedTransaction {
        txid,
        inputs,
        outputs,
    })
}

/// Returns the P2PKH sender address claimed by `script_sig`.
///
/// Only the exact P2PKH spend shape matches: a DER-shaped signature push
/// (`0x30` tag, at most 73 bytes with the sighash byte) followed by a 33 or
/// 65 byte public key push, and nothing else. This keeps a P2SH redeem script
/// whose last push happens to parse as a key from yielding an unrelated address.
fn input_address_from_script_sig(script_sig: &ScriptBuf, network: Network) -> Option<Address> {
    let mut sig: Option<&[u8]> = None;
    let mut pubkey_bytes: Option<&[u8]> = None;
    for instruction in script_sig.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(bytes)) => match (sig, pubkey_bytes) {
                (None, None) => sig = Some(bytes.as_bytes()),
                (Some(_), None) => pubkey_bytes = Some(bytes.as_bytes()),
                _ => return None,
            },
            _ => return None,
        }
    }
    let (sig, pubkey_bytes) = (sig?, pubkey_bytes?);
    if sig.is_empty() || sig[0] != 0x30 || sig.len() > 73 {
        return None;
    }
    let pubkey = PublicKey::from_slice(pubkey_bytes).ok()?;
    Some(Address::p2pkh(&pubkey, network))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{p2pkh_spend_tx, test_pubkey, tx_with_script_sig};
    use dashcore::blockdata::script::Builder;
    use dashcore::consensus::serialize;

    fn decode(tx: &Transaction, network: Network) -> DecodedTransaction {
        decode_transaction(&serialize(tx), network).expect("decodes")
    }

    #[test]
    fn should_decode_outputs_with_addresses_and_values() {
        let addr = Address::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072, 20_002]);
        let decoded = decode(&tx, Network::Testnet);

        assert_eq!(decoded.txid, tx.txid());
        assert_eq!(decoded.outputs.len(), 2);
        assert_eq!(decoded.outputs[0].address, Some(addr.clone()));
        assert_eq!(decoded.outputs[0].value, 151_072);
        assert_eq!(decoded.outputs[0].script_pubkey, addr.script_pubkey());
        assert_eq!(decoded.outputs[1].address, Some(addr.clone()));
        assert_eq!(decoded.outputs[1].value, 20_002);

        // The same script renders as a mainnet address on mainnet.
        let mainnet = decode(&tx, Network::Mainnet).outputs[0]
            .address
            .clone()
            .unwrap();
        assert_ne!(mainnet, addr);
        assert!(mainnet.to_string().starts_with('X'), "{mainnet}");

        let (tx, _) = p2pkh_spend_tx();
        let op_return = &decode(&tx, Network::Testnet).outputs[1];
        assert!(op_return.address.is_none());
        assert!(
            !op_return.script_pubkey.is_empty(),
            "script bytes still present"
        );
    }

    #[test]
    fn should_recover_the_p2pkh_input_address_from_the_script_sig() {
        let (tx, addr) = p2pkh_spend_tx();
        let decoded = decode(&tx, Network::Testnet);

        assert_eq!(decoded.inputs.len(), 1);
        assert_eq!(
            decoded.inputs[0].previous_output,
            tx.input[0].previous_output
        );
        assert_eq!(decoded.inputs[0].sender_address, Some(addr));
    }

    #[test]
    fn should_yield_no_input_address_for_a_non_p2pkh_script_sig() {
        let addr = Address::dummy(Network::Testnet, 3);
        let coinbase = Transaction::dummy_coinbase(&addr, 50_000);
        assert!(decode(&coinbase, Network::Testnet).inputs[0]
            .sender_address
            .is_none());

        // `Transaction::dummy` puts a lock script into the scriptSig: opcodes
        // present and a 20-byte last push.
        let lock_script_sig = Transaction::dummy(&addr, 1..2, &[1_000]);
        assert!(decode(&lock_script_sig, Network::Testnet).inputs[0]
            .sender_address
            .is_none());

        // Three pushes ending in a valid key, the P2SH redeem-script shape.
        let redeem = Builder::new()
            .push_slice([0x30u8; 71])
            .push_slice([0x01u8; 20])
            .push_key(&test_pubkey())
            .into_script();
        let decoded = decode(&tx_with_script_sig(redeem), Network::Testnet);
        assert!(decoded.inputs[0].sender_address.is_none());

        // Two pushes, but the first has no DER `0x30` tag.
        let not_a_signature = Builder::new()
            .push_slice([0xAAu8; 10])
            .push_key(&test_pubkey())
            .into_script();
        let decoded = decode(&tx_with_script_sig(not_a_signature), Network::Testnet);
        assert!(decoded.inputs[0].sender_address.is_none());
    }

    #[test]
    fn should_reject_bytes_that_are_not_exactly_one_transaction() {
        let (tx, _) = p2pkh_spend_tx();
        let mut trailing = serialize(&tx);
        trailing.extend_from_slice(&[0xDE, 0xAD]);
        assert!(decode_transaction(&trailing, Network::Testnet).is_err());
        assert!(decode_transaction(&[0xFFu8; 16], Network::Testnet).is_err());
        assert!(decode_transaction(&[], Network::Testnet).is_err());
    }
}
