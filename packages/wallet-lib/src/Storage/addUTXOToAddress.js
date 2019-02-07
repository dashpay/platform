const { cloneDeep } = require('lodash');
const { InvalidUTXO } = require('../errors');
const { is } = require('../utils');

/**
 * Allow to add a specific UTXO to a specific address
 * @param utxo - A valid UTXO
 * @param address - A valid Address.
 * @return {boolean}
 */
const addUTXOToAddress = function (utxo, address) {
  if (!is.address(address)) throw new Error('Invalid address');
  if (is.arr(utxo)) {
    utxo.forEach((_utxo) => {
      this.addUTXOToAddress(_utxo, address);
    });
    return false;
  }
  if (!is.utxo(utxo)) throw new InvalidUTXO(utxo);
  const searchAddr = this.searchAddress(address);

  if (searchAddr.found) {
    const newAddr = cloneDeep(searchAddr.result);
    if (!newAddr.transactions.includes(utxo.txid)) {
      newAddr.transactions.push(utxo.txid);
    }

    // If the received utxo does not exist
    const utxoKey = `${utxo.txid}-${utxo.outputIndex}`;

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
