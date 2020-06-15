const IdentityReplaceError = require('../../../errors/IndentityIdReplaceError');

/**
 *
 * @param {string} walletId
 * @param {string} identityId
 * @param {number} identityIndex
 */
function insertIdentityAtIndex(walletId, identityId, identityIndex) {
  if (!this.store.wallets[walletId].identityIds) {
    this.store.wallets[walletId].identityIds = [];
  }

  const existingId = this.getIdentityIdByIndex(walletId, identityIndex);

  if (Boolean(existingId) && existingId !== identityId) {
    throw new IdentityReplaceError(`Trying to replace identity at index ${identityIndex}`);
  }

  this.store.wallets[walletId].identityIds[identityIndex] = identityId;
  this.lastModified = Date.now();
}

module.exports = insertIdentityAtIndex;
