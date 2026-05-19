/**
 *
 * @return {string[]}
 */
function getIdentityIds() {
  return this.storage
    .getWalletStore(this.walletId)
    .getIndexedIdentityIds()
    .filter(Boolean);
}

export default getIdentityIds;