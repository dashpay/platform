/**
 * @param sinonSandbox
 * @return {{
 *   fetchDataContract: *,
 *   storeDataContract: *,
 *   fetchDocuments: *,
 *   createDocument: *,
 *   updateDocument: *,
 *   removeDocument: *,
 *   fetchTransaction: *,
 *   fetchIdentity: *,
 *   createIdentity: *,
 *   updateIdentity: *,
 *   verifyInstantLock: *,
 *   fetchSMLStore: *,
 * }}
 */
module.exports = function createStateRepositoryMock(sinonSandbox) {
  return {
    fetchDataContract: sinonSandbox.stub(),
    storeDataContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    createDocument: sinonSandbox.stub(),
    updateDocument: sinonSandbox.stub(),
    removeDocument: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
    fetchIdentity: sinonSandbox.stub(),
    createIdentity: sinonSandbox.stub(),
    updateIdentity: sinonSandbox.stub(),
    fetchLatestPlatformBlockHeader: sinonSandbox.stub(),
    storeIdentityPublicKeyHashes: sinonSandbox.stub(),
    fetchIdentityIdsByPublicKeyHashes: sinonSandbox.stub(),
    verifyInstantLock: sinonSandbox.stub(),
    markAssetLockTransactionOutPointAsUsed: sinonSandbox.stub(),
    verifyChainLockHeight: sinonSandbox.stub(),
    isAssetLockTransactionOutPointAlreadyUsed: sinonSandbox.stub(),
    fetchSMLStore: sinonSandbox.stub(),
  };
};
