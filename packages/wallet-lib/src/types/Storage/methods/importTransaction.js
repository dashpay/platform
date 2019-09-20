const { cloneDeep } = require('lodash');
const { InvalidTransactionObject } = require('../../../errors');
const { is, dashToDuffs } = require('../../../utils');
const { SECURE_TRANSACTION_CONFIRMATIONS_NB, UNCONFIRMED_TRANSACTION_STATUS_CODE } = require('../../../CONSTANTS');
const { FETCHED_UNCONFIRMED_TRANSACTION, FETCHED_CONFIRMED_TRANSACTION } = require('../../../EVENTS');
/**
 * This method is used to import a transaction in Store.
 * @param transaction - A valid Transaction
 */
const importTransaction = function importTransaction(transaction) {
  const self = this;
  if (!is.transactionObj(transaction)) throw new InvalidTransactionObject(transaction);
  const { store, network } = this;

  const transactionStore = store.transactions;
  const currBlockheight = store.chains[network].blockheight;
  const transactionsIds = Object.keys(transactionStore);

  if (!transactionsIds.includes(transaction.txid)) {
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


    const secureBlockheight = transaction.blockheight + SECURE_TRANSACTION_CONFIRMATIONS_NB;
    const isSecureTx = (
      transaction.blockheight !== UNCONFIRMED_TRANSACTION_STATUS_CODE
      && currBlockheight >= secureBlockheight
    );

    const eventName = (isSecureTx)
      ? FETCHED_CONFIRMED_TRANSACTION
      : FETCHED_UNCONFIRMED_TRANSACTION;

    self.announce(eventName, { transaction });
  } else {
    this.updateTransaction(transaction);
  }
};
module.exports = importTransaction;
