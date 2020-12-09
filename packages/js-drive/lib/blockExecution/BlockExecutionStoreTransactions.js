const MongoDBTransaction = require('../mongoDb/MongoDBTransaction');
const DocumentsIndexedTransaction = require('../document/DocumentsIndexedTransaction');
const BlockExecutionStoreTransactionIsAlreadyStartedError = require('./errors/BlockExecutionStoreTransactionIsAlreadyStartedError');
const BlockExecutionStoreTransactionIsNotStartedError = require('./errors/BlockExecutionStoreTransactionIsNotStartedError');
const BlockExecutionStoreTransactionIsNotDefinedError = require('./errors/BlockExecutionStoreTransactionIsNotDefinedError');

class BlockExecutionStoreTransactions {
  /**
   *
   * @param {MerkDbStore} commonStore
   * @param {MerkDbStore} identitiesStore
   * @param {MerkDbStore} documentsStore
   * @param {MerkDbStore} dataContractsStore
   * @param {MerkDbStore} publicKeyToIdentityIdStore
   * @param {MerkDbStore} spentAssetLockTransactionsStore
   * @param {connectToMongoDB} connectToDocumentMongoDB
   */
  constructor(
    commonStore,
    identitiesStore,
    documentsStore,
    dataContractsStore,
    publicKeyToIdentityIdStore,
    spentAssetLockTransactionsStore,
    connectToDocumentMongoDB,
  ) {
    const documentsTransaction = new DocumentsIndexedTransaction(
      documentsStore.createTransaction(),
      new MongoDBTransaction(connectToDocumentMongoDB),
    );

    this.commonStore = commonStore;
    this.identitiesStore = identitiesStore;
    this.documentsStore = documentsStore;
    this.dataContractsStore = dataContractsStore;
    this.publicKeyToIdentityIdStore = publicKeyToIdentityIdStore;
    this.spentAssetLockTransactionsStore = spentAssetLockTransactionsStore;
    this.connectToDocumentMongoDB = connectToDocumentMongoDB;

    this.transactions = {
      common: commonStore.createTransaction(),
      identities: identitiesStore.createTransaction(),
      documents: documentsTransaction,
      dataContracts: dataContractsStore.createTransaction(),
      publicKeyToIdentityId: publicKeyToIdentityIdStore.createTransaction(),
      assetLockTransactions: spentAssetLockTransactionsStore.createTransaction(),
    };

    this.isTransactionsStarted = false;
  }

  /**
   * Start transactions
   */
  async start() {
    if (this.isTransactionsStarted) {
      throw new BlockExecutionStoreTransactionIsAlreadyStartedError();
    }

    await Promise.all(
      Object.values(this.transactions).map((t) => t.start()),
    );

    this.isTransactionsStarted = true;
  }

  /**
   * Commit transactions
   *
   * @return {Promise<void>}
   */
  async commit() {
    if (!this.isTransactionsStarted) {
      throw new BlockExecutionStoreTransactionIsNotStartedError();
    }

    await Promise.all(
      Object
        .values(this.transactions)
        .map((transaction) => transaction.commit()),
    );

    this.isTransactionsStarted = false;
  }

  /**
   * Abort transactions
   *
   * @return {Promise<void>}
   */
  async abort() {
    if (!this.isTransactionsStarted) {
      throw new BlockExecutionStoreTransactionIsNotStartedError();
    }

    await Promise.all(
      Object
        .values(this.transactions)
        .map((transaction) => transaction.abort()),
    );

    this.isTransactionsStarted = false;
  }

  /**
   * Are transactions started
   *
   * @returns {boolean}
   */
  isStarted() {
    return this.isTransactionsStarted;
  }

  /**
   * Get transaction by name
   *
   * @return {MerkDbTransaction|MongoDBTransaction}
   */
  getTransaction(name) {
    if (!this.transactions[name]) {
      throw new BlockExecutionStoreTransactionIsNotDefinedError(name);
    }

    return this.transactions[name];
  }

  /**
   * Return transactions as plain objects
   *
   * @return {RawStoreTransaction}
   */
  toObject() {
    return Object.entries(this.transactions)
      .reduce((transactions, [name, transaction]) => (
        {
          ...transactions,
          [name]: transaction.toObject(),
        }
      ), {});
  }

  /**
   * Populate transactions from transactions object
   *
   * @param {RawStoreTransaction} transactionsObject
   */
  async populateFromObject(transactionsObject) {
    for (const name of Object.keys(transactionsObject)) {
      await this.transactions[name].populateFromObject(transactionsObject[name]);
    }
  }
}

/**
 * @typedef {Object} RawStoreTransaction
 * @property {Object} updates
 * @property {Object} deletes
 */

/**
 * @typedef {Object} RawBlockExecutionStoreTransactions
 * @property {RawStoreTransaction} common
 * @property {RawStoreTransaction} identities
 * @property {RawStoreTransaction} documents
 * @property {RawStoreTransaction} dataContracts
 * @property {RawStoreTransaction} publicKeyToIdentityId
 * @property {RawStoreTransaction} assetLockTransactions
 */

module.exports = BlockExecutionStoreTransactions;
