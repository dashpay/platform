const { cloneDeep } = require('lodash');
const { InvalidDashcoreTransaction } = require('../../../errors');
const { is } = require('../../../utils');
const { FETCHED_UNCONFIRMED_TRANSACTION, FETCHED_CONFIRMED_TRANSACTION } = require('../../../EVENTS');
/**
 * This method is used to import a transaction in Store.
 * @param transaction - A valid Transaction
 */
const importTransaction = function importTransaction(transaction) {
  const self = this;

  if (!is.dashcoreTransaction(transaction)) throw new InvalidDashcoreTransaction(transaction);
  const { store, network } = this;

  const transactionStore = store.transactions;
  const currBlockheight = store.chains[network].blockHeight;
  const transactionsIds = Object.keys(transactionStore);

  if (!transactionsIds.includes(transaction.hash)) {
    // eslint-disable-next-line no-param-reassign
    this.store.transactions[transaction.hash] = transaction;
    // We should now also check if it concern one of our address
    // VIN
    const vins = transaction.inputs;
    vins.forEach((vin) => {
      if (vin.script) {
        const search = self.searchAddress(vin.script.toAddress(network).toString());
        if (search.found) {
          const newAddr = cloneDeep(search.result);
          const utxoKey = `${vin.prevTxId.toString('hex')}-${vin.outputIndex}`;
          delete newAddr[utxoKey];
          if (!newAddr.transactions.includes(transaction.hash)) {
            newAddr.transactions.push(transaction.hash);
            newAddr.used = true;
            self.updateAddress(newAddr, search.walletId);
          }
        }
      }
    });

    // VOUT
    const vouts = transaction.outputs;
    // For all output, we need to insert the utxo + associate the tx to addr.transactions
    vouts.forEach((vout, voutIndex) => {
      const search = self.searchAddress(vout.script.toAddress(network).toString());
      if (search.found) {
        self.addUTXOToAddress(vout, search.result.address, transaction.hash, voutIndex);
      }
    });
    this.lastModified = +new Date();

    const blockHeight = transaction.nLockTime;
    const isSecureTx = (blockHeight <= currBlockheight);

    const eventName = (isSecureTx)
      ? FETCHED_CONFIRMED_TRANSACTION
      : FETCHED_UNCONFIRMED_TRANSACTION;

    self.announce(eventName, { transaction });
  } else {
    this.updateTransaction(transaction);
  }
};
module.exports = importTransaction;
