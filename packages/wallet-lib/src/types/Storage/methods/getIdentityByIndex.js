/**
 *
 * @param {string} walletId
 * @param {number} identityIndex
 * @return {string|undefined}
 */
function getIdentityIdByIndex(walletId, identityIndex) {
  return this.store.wallets[walletId].identityIds[identityIndex];
}

module.exports = getIdentityIdByIndex;
