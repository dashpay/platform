/**
 * @param sinonSandbox
 * @return {{
 *   fetchDataContract: *,
 *   storeDataContract: *,
 *   fetchDocuments: *,
 *   storeDocument: *,
 *   removeDocument: *,
 *   fetchTransaction: *,
 *   fetchIdentity: *,
 *   storeIdentity: *,
 *   fetchSMLStore: *,
 * }}
 */
module.exports = function createStateRepositoryMock(sinonSandbox) {
  return {
    fetchDataContract: sinonSandbox.stub(),
    storeDataContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    storeDocument: sinonSandbox.stub(),
    removeDocument: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
    fetchIdentity: sinonSandbox.stub(),
    storeIdentity: sinonSandbox.stub(),
    fetchLatestPlatformBlockHeader: sinonSandbox.stub(),
    storeIdentityPublicKeyHashes: sinonSandbox.stub(),
    fetchIdentityIdsByPublicKeyHashes: sinonSandbox.stub(),
    fetchSMLStore: sinonSandbox.stub(),
    storeAssetLockTransactionOutPoint: sinonSandbox.stub(),
    checkAssetLockTransactionOutPointExists: sinonSandbox.stub(),
  };
};
