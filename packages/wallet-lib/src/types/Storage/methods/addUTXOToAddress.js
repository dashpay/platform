const { cloneDeep } = require('lodash');
const { Transaction } = require('@dashevo/dashcore-lib');
const { InvalidUTXO } = require('../../../errors');
const { is } = require('../../../utils');

/**
 * Allow to add a specific UTXO to a specific address
 * @param output - A valid Output
 * @param address - A valid Address.
 * @param tx - A valid TXID where the utxo occured
 * @param outputIndex - The index of the utxo in the tx
 * @return {boolean}
 */
const addUTXOToAddress = function (output, address, txid, outputIndex) {
  if (!is.address(address)) throw new Error('Invalid address');
  if (is.arr(output)) {
    output.forEach((_output) => {
      this.addUTXOToAddress(_output, address);
    });
    return false;
  }
  // Right now, we do not receive txid from getUTXO of DAPIClient
  // eslint-disable-next-line no-param-reassign
  output.txid = txid;
  // eslint-disable-next-line no-param-reassign
  output.outputIndex = outputIndex;
  // eslint-disable-next-line no-param-reassign
  output.address = address;
  const utxo = new Transaction.UnspentOutput(output);
  if (!is.output(utxo)) throw new InvalidUTXO(utxo);

  const searchAddr = this.searchAddress(address);

  if (searchAddr.found) {
    const newAddr = cloneDeep(searchAddr.result);
    if (!newAddr.transactions.includes(txid)) {
      newAddr.transactions.push(txid);
    }

    // If the received utxo does not exist
    const utxoKey = `${txid}-${outputIndex}`;

    if (!!newAddr.utxos[utxoKey] === false) {
      newAddr.utxos[utxoKey] = utxo;
      newAddr.used = true;
      this.updateAddress(newAddr, searchAddr.walletId);
      return true;
    }
  }
  return false;
};
module.exports = addUTXOToAddress;
