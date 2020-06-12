/**
 *
 * @param {string} walletId
 * @return {Array<string|undefined>}
 */
function getIndexedIdentityIds(walletId) {
  return this.store.wallets[walletId].identityIds;
}

module.exports = getIndexedIdentityIds;
