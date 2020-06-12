/**
 *
 * @return {Promise<number>}
 */
async function getUnusedIdentityIndex() {
  // Force identities sync before return unused index
  await this.getWorker('IdentitySyncWorker').execWorker();

  const identityIds = this.storage.getIndexedIdentityIds(this.walletId);

  const firstMissingIndex = identityIds.findIndex((identityId) => !identityId);

  return firstMissingIndex > -1 ? firstMissingIndex : identityIds.length;
}

module.exports = getUnusedIdentityIndex;
