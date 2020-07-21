const { InvalidAddress, InvalidTransactionObject, StorageUnableToAddTransaction } = require('../../../errors');
const { is } = require('../../../utils');

/**
 * Add a new transaction to an address (push a tx)
* @param {TransactionInfo} tx
* @param {String} address
* @return {boolean}
  */
const addNewTxToAddress = function (tx, address) {
  if (!is.address(address)) throw new InvalidAddress(address);
  if (!is.transactionObj(tx)) throw new InvalidTransactionObject(tx);

  const searchAddr = this.searchAddress(address);
  const { walletId } = searchAddr;

  if (tx.address && tx.txid) {
    const { type, path, found } = this.searchAddress(tx.address);
    if (!found) {
      throw new StorageUnableToAddTransaction(tx);
    }
    this.store.wallets[walletId].addresses[type][path].transactions.push(tx.txid);
    // Because of the unclear state will force a refresh
    this.store.wallets[walletId].addresses[type][path].fetchedLast = 0;
    this.lastModified = +new Date();
    return true;
  }
  throw new StorageUnableToAddTransaction(tx);
};
module.exports = addNewTxToAddress;
