const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const DataContractCacheItem = require('../dataContract/DataContractCacheItem');

/**
 * @implements StateRepository
 */
class CachedStateRepositoryDecorator {
  /**
   * @param {DriveStateRepository} stateRepository
   * @param {LRUCache} dataContractCache
   */
  constructor(
    stateRepository,
    dataContractCache,
  ) {
    this.stateRepository = stateRepository;
    this.contractCache = dataContractCache;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id, executionContext = undefined) {
    return this.stateRepository.fetchIdentity(id, executionContext);
  }

  /**
   * Create identity
   *
   * @param {Identity} identity
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async createIdentity(identity, executionContext = undefined) {
    return this.stateRepository.createIdentity(identity, executionContext);
  }

  /**
   * Update identity
   *
   * @param {Identity} identity
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async updateIdentity(identity, executionContext = undefined) {
    return this.stateRepository.updateIdentity(identity, executionContext);
  }

  /**
   * Store public key hashes for an identity id
   *
   * @param {Identifier} identityId
   * @param {Buffer[]} publicKeyHashes
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async storeIdentityPublicKeyHashes(identityId, publicKeyHashes, executionContext = undefined) {
    return this.stateRepository.storeIdentityPublicKeyHashes(
      identityId,
      publicKeyHashes,
      executionContext,
    );
  }

  /**
   * Fetch identity ids mapped by related public keys
   * using public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Array<Identifier[]>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes, executionContext = undefined) {
    return this.stateRepository.fetchIdentityIdsByPublicKeyHashes(
      publicKeyHashes,
      executionContext,
    );
  }

  /**
   * Store spent asset lock transaction
   *
   * @param {Buffer} outPointBuffer
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<void>}
   */
  async markAssetLockTransactionOutPointAsUsed(outPointBuffer, executionContext = undefined) {
    return this.stateRepository.markAssetLockTransactionOutPointAsUsed(
      outPointBuffer,
      executionContext,
    );
  }

  /**
   * Check if spent asset lock transaction is stored
   *
   * @param {Buffer} outPointBuffer
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<boolean>}
   */
  async isAssetLockTransactionOutPointAlreadyUsed(outPointBuffer, executionContext = undefined) {
    return this.stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
      outPointBuffer,
      executionContext,
    );
  }

  /**
   * Fetch Data Contract by ID
   *
   * @param {Identifier} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id, executionContext = undefined) {
    const idString = id.toString();

    let cacheItem = this.contractCache.get(idString);

    if (cacheItem) {
      if (executionContext) {
        executionContext.addOperation(...cacheItem.getOperations());
      }

      return cacheItem.getDataContract();
    }

    const isolatedExecutionContext = new StateTransitionExecutionContext();

    const dataContract = await this.stateRepository.fetchDataContract(id, isolatedExecutionContext);

    if (executionContext) {
      executionContext.addOperation(...isolatedExecutionContext.getOperations());
    }

    if (dataContract !== null) {
      cacheItem = new DataContractCacheItem(
        dataContract,
        isolatedExecutionContext.getOperations(),
      );

      this.contractCache.set(idString, cacheItem);
    }

    return dataContract;
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async storeDataContract(dataContract, executionContext = undefined) {
    return this.stateRepository.storeDataContract(dataContract, executionContext);
  }

  /**
   * Fetch Documents by contract ID and type
   *
   * @param {Identifier} contractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Document[]>}
   */
  async fetchDocuments(contractId, type, options = {}, executionContext = undefined) {
    return this.stateRepository.fetchDocuments(contractId, type, options, executionContext);
  }

  /**
   * Create document
   *
   * @param {Document} document
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async createDocument(document, executionContext = undefined) {
    return this.stateRepository.createDocument(document, executionContext);
  }

  /**
   * Update document
   *
   * @param {Document} document
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async updateDocument(document, executionContext = undefined) {
    return this.stateRepository.updateDocument(document, executionContext);
  }

  /**
   * Remove document
   *
   * @param {DataContract} dataContract
   * @param {string} type
   * @param {Identifier} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async removeDocument(dataContract, type, id, executionContext = undefined) {
    return this.stateRepository.removeDocument(dataContract, type, id, executionContext);
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Object|null>}
   */
  async fetchTransaction(id, executionContext = undefined) {
    return this.stateRepository.fetchTransaction(id, executionContext);
  }

  /**
   * Fetch the latest platform block height
   *
   * @return {Promise<Long>}
   */
  async fetchLatestPlatformBlockHeight() {
    return this.stateRepository.fetchLatestPlatformBlockHeight();
  }

  /**
   * Fetch the latest platform block time
   *
   * @return {Promise<protobuf.Timestamp>}
   */
  async fetchLatestPlatformBlockTime() {
    return this.stateRepository.fetchLatestPlatformBlockTime();
  }

  /**
   * Fetch the latest platform core chainlocked height
   *
   * @return {Promise<number>}
   */
  async fetchLatestPlatformCoreChainLockedHeight() {
    return this.stateRepository.fetchLatestPlatformCoreChainLockedHeight();
  }

  /**
   * Verify instant lock
   *
   * @param {InstantLock} instantLock
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<boolean>}
   */
  async verifyInstantLock(instantLock, executionContext = undefined) {
    return this.stateRepository.verifyInstantLock(instantLock, executionContext);
  }

  /**
   * Fetch Simplified Masternode List Store
   *
   * @return {Promise<SimplifiedMNListStore>}
   */
  async fetchSMLStore() {
    return this.stateRepository.fetchSMLStore();
  }

  /**
   * Fetch latest withdrawal transaction index
   *
   * @returns {Promise<number>}
   */
  async fetchLatestWithdrawalTransactionIndex() {
    return this.stateRepository.fetchLatestWithdrawalTransactionIndex();
  }

  /**
   * Enqueue withdrawal transaction bytes into the queue
   *
   * @param {number} index
   * @param {Buffer} transactionBytes
   *
   * @returns {Promise<void>}
   */
  async enqueueWithdrawalTransaction(index, transactionBytes) {
    return this.stateRepository.enqueueWithdrawalTransaction(
      index,
      transactionBytes,
    );
  }
}

module.exports = CachedStateRepositoryDecorator;
