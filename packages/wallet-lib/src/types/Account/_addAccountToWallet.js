/**
 * Add when not existing a element account in a parent wallet
 * @param account
 * @param wallet
 */
// eslint-disable-next-line no-underscore-dangle
const _addAccountToWallet = function addAccountToWallet(account, wallet) {
  const { accounts } = wallet;

  const existAlready = accounts.filter((el) => el.accountIndex === wallet.accountIndex).length > 0;
  if (!existAlready) {
    wallet.accounts.push(account);
  }
};
module.exports = _addAccountToWallet;
