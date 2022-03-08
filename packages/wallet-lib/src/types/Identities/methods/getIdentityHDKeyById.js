/**
 *
 * @param {string} identityId
 * @param {number} keyIndex
 * @return {HDPrivateKey}
 */
function getIdentityHDKeyById(identityId, keyIndex) {
  const identityIndex = this.storage
    .getWalletStore(this.walletId)
    .getIndexedIdentityIds()
    .indexOf(identityId);

  if (identityIndex === -1) {
    throw new Error(`Identity with ID ${identityId} is not associated with wallet, or it's not synced`);
  }

  return this.getIdentityHDKeyByIndex(identityIndex, keyIndex);
}

module.exports = getIdentityHDKeyById;
