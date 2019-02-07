const importSingleAddress = function (singleAddress, walletId) {
  const type = singleAddress.constructor.name;
  if (!walletId) throw new Error('Expected walletId to import single address');
  if (!this.searchWallet(walletId).found) {
    this.createWallet(walletId);
  }
  const accList = this.store.wallets[walletId].accounts;

  if (type === 'Object') {
    if (singleAddress.path) {
      accList[singleAddress.path] = (singleAddress);
      this.lastModified = +new Date();
    }
  } else if (type === 'Array') {
    throw new Error('Not implemented. Please create an issue on github if needed.');
  } else {
    throw new Error('Invalid account. Cannot import.');
  }
  return true;
};
module.exports = importSingleAddress;
