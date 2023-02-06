/**
 * @typedef {createStateRepositoryMock}
 * @param sinonSandbox
 * @return {{
 *    fetchDataContract: *,
 *    createDataContract: *,
 *    updateDataContract: *,
 *     fetchDocuments: *,
 *     createDocument: *,
 *     updateDocument: *,
 *     removeDocument: *,
 *     fetchTransaction: *,
 *     fetchIdentity: *,
 *     createIdentity: *,
 *     updateIdentityRevision: *,
 *     disableIdentityKeys: *,
 *     addKeysToIdentity: *,
 *     fetchIdentityBalance: *,
 *     fetchIdentityBalanceWithDebt: *,
 *     addToIdentityBalance: *,
 *     addToSystemCredits: *,
 *     fetchLatestPlatformBlockHeight: *,
 *     fetchLatestPlatformCoreChainLockedHeight: *,
 *     verifyInstantLock: *,
 *     markAssetLockTransactionOutPointAsUsed: *,
 *     verifyChainLockHeight: *,
 *     isAssetLockTransactionOutPointAlreadyUsed: *,
 *     fetchSMLStore: *,
 *     fetchLatestWithdrawalTransactionIndex: *,
 *     enqueueWithdrawalTransaction: *,
 *     fetchLatestPlatformBlockTime: *,
 * }}
 */
module.exports = function createStateRepositoryMock(sinonSandbox) {
  const stateRepository = {
    fetchDataContract: sinonSandbox.stub(),
    createDataContract: sinonSandbox.stub(),
    updateDataContract: sinonSandbox.stub(),
    fetchDocuments: sinonSandbox.stub(),
    createDocument: sinonSandbox.stub(),
    updateDocument: sinonSandbox.stub(),
    removeDocument: sinonSandbox.stub(),
    fetchTransaction: sinonSandbox.stub(),
    fetchIdentity: sinonSandbox.stub(),
    createIdentity: sinonSandbox.stub(),
    addKeysToIdentity: sinonSandbox.stub(),
    disableIdentityKeys: sinonSandbox.stub(),
    updateIdentityRevision: sinonSandbox.stub(),
    addToIdentityBalance: sinonSandbox.stub(),
    fetchIdentityBalance: sinonSandbox.stub(),
    fetchIdentityBalanceWithDebt: sinonSandbox.stub(),
    addToSystemCredits: sinonSandbox.stub(),
    fetchLatestPlatformBlockHeight: sinonSandbox.stub(),
    fetchLatestPlatformCoreChainLockedHeight: sinonSandbox.stub(),
    verifyInstantLock: sinonSandbox.stub(),
    markAssetLockTransactionOutPointAsUsed: sinonSandbox.stub(),
    verifyChainLockHeight: sinonSandbox.stub(),
    isAssetLockTransactionOutPointAlreadyUsed: sinonSandbox.stub(),
    fetchSMLStore: sinonSandbox.stub(),
    fetchLatestWithdrawalTransactionIndex: sinonSandbox.stub(),
    enqueueWithdrawalTransaction: sinonSandbox.stub(),
    fetchLatestPlatformBlockTime: sinonSandbox.stub(),
    calculateStorageFeeDistributionAmountAndLeftovers: sinonSandbox.stub(),
  };

  Object.values(stateRepository).forEach((method) => method.resolves());

  return stateRepository;
};
