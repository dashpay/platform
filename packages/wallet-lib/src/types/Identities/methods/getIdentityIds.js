/**
 *
 * @return {string[]}
 */
function getIdentityIds() {
  return this.storage.getIndexedIdentityIds(this.walletId).filter(Boolean);
}

module.exports = getIdentityIds;
