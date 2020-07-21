/**
 * Import an array of accounts or a account object to the store
 * @param {Account|[Account]} accounts
 * @param {string} walletId
 * @return {boolean}
 */
const importAccounts = function (accounts, walletId) {
  if (!walletId) throw new Error('Expected walletId to import addresses');
  if (!this.searchWallet(walletId).found) {
    this.createWallet(walletId);
  }
  const accList = this.store.wallets[walletId].accounts;

  const type = accounts.constructor.name;
  if (type === 'Object') {
    if (accounts.path) {
      if (!accList[accounts.path]) {
        accList[accounts.path] = accounts;
        this.lastModified = +new Date();
      }
    } else {
      const accountsPaths = Object.keys(accounts);
      accountsPaths.forEach((path) => {
        const el = accounts[path];
        if (el.path) {
          if (!accList[el.path]) {
            accList[el.path] = el;
            this.lastModified = +new Date();
          }
        }
      });
    }
  } else if (type === 'Array') {
    accounts.forEach((account) => {
      importAccounts.call(this, account, walletId);
    });
  } else if (type === 'Account') {
    const accObj = {
      label: accounts.label,
      path: accounts.BIP44PATH,
      network: accounts.network,
    };
    return importAccounts.call(this, accObj, walletId);
  } else {
    throw new Error('Invalid account. Cannot import.');
  }
  return true;
};
module.exports = importAccounts;
