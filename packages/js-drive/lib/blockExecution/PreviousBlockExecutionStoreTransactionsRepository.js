const cbor = require('cbor');
const BlockExecutionStoreTransactions = require('./BlockExecutionStoreTransactions');
const BlockExecutionStoreTransactionIsNotStartedError = require('./errors/BlockExecutionStoreTransactionIsNotStartedError');

class PreviousBlockExecutionStoreTransactionsRepository {
  /**
   * @param {FileDb} previousBlockExecutionTransactionDB
   * @param {MerkDbStore} previousCommonStore
   * @param {MerkDbStore} previousIdentitiesStore
   * @param {MerkDbStore} previousDocumentsStore
   * @param {MerkDbStore} previousDataContractsStore
   * @param {MerkDbStore} previousPublicKeyToIdentityIdStore
   * @param {MerkDbStore} previousSpentAssetLockTransactionsStore
   * @param {connectToMongoDB} connectToDocumentMongoDB
   */
  constructor(
    previousBlockExecutionTransactionDB,
    previousCommonStore,
    previousIdentitiesStore,
    previousDocumentsStore,
    previousDataContractsStore,
    previousPublicKeyToIdentityIdStore,
    previousSpentAssetLockTransactionsStore,
    connectToDocumentMongoDB,
  ) {
    this.blockExecutionTransactionDB = previousBlockExecutionTransactionDB;

    this.commonStore = previousCommonStore;
    this.identitiesStore = previousIdentitiesStore;
    this.documentsStore = previousDocumentsStore;
    this.dataContractsStore = previousDataContractsStore;
    this.publicKeyToIdentityIdStore = previousPublicKeyToIdentityIdStore;
    this.spentAssetLockTransactionsStore = previousSpentAssetLockTransactionsStore;
    this.connectToDocumentMongoDB = connectToDocumentMongoDB;
  }

  /**
   * @param {BlockExecutionStoreTransactions} storeTransactions
   *
   * @return {void}
   */
  async store(storeTransactions) {
    if (!storeTransactions.isStarted()) {
      throw new BlockExecutionStoreTransactionIsNotStartedError();
    }

    const serializedTransactions = cbor.encode(storeTransactions.toObject());

    await this.blockExecutionTransactionDB.set(serializedTransactions);
  }

  /**
   * Fetch BlockExecutionStoreTransactions
   *
   * @return {BlockExecutionStoreTransactions|null}
   */
  async fetch() {
    const serializedTransactions = await this.blockExecutionTransactionDB.get();

    if (!serializedTransactions) {
      return null;
    }

    const newBlockExecutionStoreTransactions = new BlockExecutionStoreTransactions(
      this.commonStore,
      this.identitiesStore,
      this.documentsStore,
      this.dataContractsStore,
      this.publicKeyToIdentityIdStore,
      this.spentAssetLockTransactionsStore,
      this.connectToDocumentMongoDB,
    );

    await newBlockExecutionStoreTransactions.start();

    const plainObjectTransactions = cbor.decode(serializedTransactions);

    await newBlockExecutionStoreTransactions.populateFromObject(plainObjectTransactions);

    return newBlockExecutionStoreTransactions;
  }

  /**
   * Clear DB state
   *
   * @return {void}
   */
  async clear() {
    await this.blockExecutionTransactionDB.clear();
  }
}

module.exports = PreviousBlockExecutionStoreTransactionsRepository;
