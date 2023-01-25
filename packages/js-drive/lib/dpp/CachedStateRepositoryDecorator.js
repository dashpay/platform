/**
 * @implements StateRepository
 */
class CachedStateRepositoryDecorator {
  /**
   * @param {DriveStateRepository} stateRepository
   */
  constructor(
    stateRepository,
  ) {
    this.stateRepository = stateRepository;
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
   * Add keys to identity
   *
   * @param {Identifier} identityId
   * @param {IdentityPublicKey[]} keys
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async addKeysToIdentity(identityId, keys, executionContext = undefined) {
    return this.stateRepository.addKeysToIdentity(identityId, keys, executionContext);
  }

  /**
   * Add to identity balance
   *
   * @param {Identifier} identityId
   * @param {number} amount
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async addToIdentityBalance(identityId, amount, executionContext = undefined) {
    return this.stateRepository.addToIdentityBalance(
      identityId,
      amount,
      executionContext,
    );
  }

  /**
   * Add to system credits
   *
   * @param {number} amount
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async addToSystemCredits(amount, executionContext = undefined) {
    return this.stateRepository.addToSystemCredits(
      amount,
      executionContext,
    );
  }

  /**
   * Disable identity keys
   *
   * @param {Identifier} identityId
   * @param {number[]} keyIds
   * @param {number} disableAt
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async disableIdentityKeys(identityId, keyIds, disableAt, executionContext = undefined) {
    return this.stateRepository.disableIdentityKeys(
      identityId,
      keyIds,
      disableAt,
      executionContext,
    );
  }

  /**
   * Update identity revision
   *
   * @param {Identifier} identityId
   * @param {number} revision
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async updateIdentityRevision(identityId, revision, executionContext = undefined) {
    return this.stateRepository.updateIdentityRevision(identityId, revision, executionContext);
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
    return this.stateRepository.fetchDataContract(
      id,
      executionContext,
    );
  }

  /**
   * Create Data Contract
   *
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async createDataContract(dataContract, executionContext = undefined) {
    return this.stateRepository.createDataContract(dataContract, executionContext);
  }

  /**
   * Update Data Contract
   *
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async updateDataContract(dataContract, executionContext = undefined) {
    return this.stateRepository.updateDataContract(dataContract, executionContext);
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
   * Fetch the latest withdrawal transaction index
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

  /**
   * Returns block time
   *
   * @returns {Promise<number>}
   */
  async fetchLatestPlatformBlockTime() {
    return this.stateRepository.fetchLatestPlatformBlockTime();
  }
}

module.exports = CachedStateRepositoryDecorator;
