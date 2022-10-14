const crypto = require('crypto');
const { Transaction, Script, PrivateKey } = require('@dashevo/dashcore-lib');

/**
 *
 * @param options
 */

const defaultUtxoOptions = {
  txId: crypto.randomBytes(32).toString('hex'),
  outputIndex: 0,
  satoshis: 1e8,
};

const mockUtxo = (options = {}) => {
  const utxo = { ...defaultUtxoOptions, ...options };
  if (!utxo.address) {
    const address = new PrivateKey('livenet').toAddress();
    utxo.address = address;
    utxo.script = Script.buildPublicKeyHashOut(address).toString();
  }

  if (!utxo.script) {
    utxo.script = Script.buildPublicKeyHashOut(utxo.address).toString();
  }

  return new Transaction.UnspentOutput(utxo);
};

const transaction = {
  mockUtxo,
};

module.exports = transaction;
