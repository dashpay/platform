const BlockExecutionStoreTransactions = require('./BlockExecutionStoreTransactions');

/**
 * @param {MerkDbStore} previousCommonStore
 * @param {MerkDbStore} previousIdentitiesStore
 * @param {MerkDbStore} previousDocumentsStore
 * @param {MerkDbStore} previousDataContractsStore
 * @param {MerkDbStore} previousPublicKeyToIdentityIdStore
 * @param {MerkDbStore} previousSpentAssetLockTransactionsStore
 * @param {connectToMongoDB} connectToDocumentMongoDB
 */
function cloneToPreviousStoreTransactionsFactory(
  previousCommonStore,
  previousIdentitiesStore,
  previousDocumentsStore,
  previousDataContractsStore,
  previousPublicKeyToIdentityIdStore,
  previousSpentAssetLockTransactionsStore,
  connectToDocumentMongoDB,
) {
  /**
   * Clone store transactions
   *
   * @returns {BlockExecutionStoreTransactions}
   */
  async function cloneToPreviousStoreTransactions(storeTransactions) {
    const newBlockExecutionStoreTransactions = new BlockExecutionStoreTransactions(
      previousCommonStore,
      previousIdentitiesStore,
      previousDocumentsStore,
      previousDataContractsStore,
      previousPublicKeyToIdentityIdStore,
      previousSpentAssetLockTransactionsStore,
      connectToDocumentMongoDB,
    );

    await newBlockExecutionStoreTransactions.start();

    await newBlockExecutionStoreTransactions.populateFromObject(storeTransactions.toObject());

    return newBlockExecutionStoreTransactions;
  }

  return cloneToPreviousStoreTransactions;
}

module.exports = cloneToPreviousStoreTransactionsFactory;
