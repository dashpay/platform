const { cloneDeep } = require('lodash');
const { InvalidTransactionObject } = require('../errors');
const { is, dashToDuffs } = require('../utils');

/**
 * This method is used to import a transaction in Store.
 * @param transaction - A valid Transaction
 */
const importTransaction = function (transaction) {
  const self = this;
  if (!is.transactionObj(transaction)) throw new InvalidTransactionObject(transaction);
  const transactionStore = this.store.transactions;
  const transactionsIds = Object.keys(transactionStore);

  if (!transactionsIds.includes[transaction.txid]) {
    // eslint-disable-next-line no-param-reassign
    transactionStore[transaction.txid] = transaction;

    // We should now also check if it concern one of our address

    // VIN
    const vins = transaction.vin;
    vins.forEach((vin) => {
      const search = self.searchAddress(vin.addr);
      if (search.found) {
        const newAddr = cloneDeep(search.result);
        if (!newAddr.transactions.includes(transaction.txid)) {
          newAddr.transactions.push(transaction.txid);
          newAddr.used = true;
          self.updateAddress(newAddr, search.walletId);
        }
      }
    });

    // VOUT
    const vouts = transaction.vout;
    vouts.forEach((vout) => {
      if (vout && vout.scriptPubKey && vout.scriptPubKey.addresses) {
        vout.scriptPubKey.addresses.forEach((addr) => {
          const search = self.searchAddress(addr);
          if (search.found) {
            const isSpent = !!vout.spentTxId;
            if (!isSpent) {
              const utxo = {
                txid: transaction.txid,
                outputIndex: vout.n,
                satoshis: dashToDuffs(parseFloat(vout.value)),
                scriptPubKey: vout.scriptPubKey.hex,
              };
              self.addUTXOToAddress(utxo, search.result.address);
            }
          }
        });
      }
    });
    this.lastModified = +new Date();
  } else {
    throw new Error('Tx already exist');
  }
};
module.exports = importTransaction;
