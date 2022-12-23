/**
 * @implements StateRepository
 */
class LoggedStateRepositoryDecorator {
  /**
   * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
   * @param {BlockExecutionContext} blockExecutionContext
   */
  constructor(
    stateRepository,
    blockExecutionContext,
  ) {
    this.stateRepository = stateRepository;
    this.blockExecutionContext = blockExecutionContext;
  }

  /**
   * @private
   * @param {string} method - state repository method name
   * @param {object} parameters - parameters of the state repository call
   * @param {object} response - response of the state repository call
   */
  log(method, parameters, response) {
    const logger = this.blockExecutionContext.getConsensusLogger();

    logger.trace({
      stateRepository: {
        method,
        parameters,
        response,
      },
    }, `StateRepository#${method}`);
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
    let response;

    try {
      response = await this.stateRepository.fetchIdentity(id, executionContext);
    } finally {
      this.log(
        'fetchIdentity',
        {
          id,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.createIdentity(identity, executionContext);
    } finally {
      this.log(
        'createIdentity',
        {
          identity,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.updateIdentity(identity, executionContext);
    } finally {
      this.log(
        'updateIdentity',
        {
          identity,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository
        .storeIdentityPublicKeyHashes(identityId, publicKeyHashes, executionContext);
    } finally {
      this.log(
        'storeIdentityPublicKeyHashes',
        {
          identityId,
          publicKeyHashes: publicKeyHashes.map((hash) => hash.toString('base64')),
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.markAssetLockTransactionOutPointAsUsed(
        outPointBuffer,
        executionContext,
      );
    } finally {
      this.log(
        'markAssetLockTransactionOutPointAsUsed',
        {
          outPointBuffer: outPointBuffer.toString('base64'),
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
        outPointBuffer,
        executionContext,
      );
    } finally {
      this.log(
        'isAssetLockTransactionOutPointAlreadyUsed',
        {
          outPointBuffer: outPointBuffer.toString('base64'),
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.fetchDataContract(id, executionContext);
    } finally {
      this.log(
        'fetchDataContract',
        {
          id,
        },
        response,
      );
    }

    return response;
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async createDataContract(dataContract, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.createDataContract(dataContract, executionContext);
    } finally {
      this.log('createDataContract', { dataContract }, response);
    }

    return response;
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async updateDataContract(dataContract, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.updateDataContract(dataContract, executionContext);
    } finally {
      this.log('updateDataContract', { dataContract }, response);
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.fetchDocuments(
        contractId,
        type,
        options,
        executionContext,
      );
    } finally {
      this.log(
        'fetchDocuments',
        {
          contractId,
          type,
          options,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.createDocument(document, executionContext);
    } finally {
      this.log(
        'createDocument',
        {
          document,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.updateDocument(document, executionContext);
    } finally {
      this.log(
        'updateDocument',
        {
          document,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.removeDocument(
        dataContract,
        type,
        id,
        executionContext,
      );
    } finally {
      this.log(
        'removeDocument',
        {
          dataContract,
          type,
          id,
        },
        response,
      );
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.fetchTransaction(id, executionContext);
    } finally {
      this.log(
        'fetchTransaction',
        {
          id,
        },
        response,
      );
    }

    return response;
  }

  /**
   * Fetch the latest platform block height
   *
   * @return {Promise<Long>}
   */
  async fetchLatestPlatformBlockHeight() {
    let response;

    try {
      response = await this.stateRepository.fetchLatestPlatformBlockHeight();
    } finally {
      this.log('fetchLatestPlatformBlockHeight', {}, response);
    }

    return response;
  }

  /**
   * Fetch the latest platform core chainlocked height
   *
   * @return {Promise<number>}
   */
  async fetchLatestPlatformCoreChainLockedHeight() {
    let response;

    try {
      response = await this.stateRepository.fetchLatestPlatformCoreChainLockedHeight();
    } finally {
      this.log('fetchLatestPlatformCoreChainLockedHeight', {}, response);
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.verifyInstantLock(instantLock, executionContext);
    } finally {
      this.log('verifyInstantLock', { instantLock }, response);
    }

    return response;
  }

  /**
   * Returns block time
   *
   * @returns {Promise<number>}
   */
  async fetchLatestPlatformBlockTime() {
    let response;

    try {
      response = await this.stateRepository.fetchLatestPlatformBlockTime();
    } finally {
      this.log('fetchLatestPlatformBlockTime', { }, response);
    }

    return response;
  }

  /**
   * Fetch latest withdrawal transaction index
   *
   * @returns {Promise<number>}
   */
  async fetchLatestWithdrawalTransactionIndex() {
    let response;

    try {
      response = await this.stateRepository.fetchLatestWithdrawalTransactionIndex();
    } finally {
      this.log('fetchLatestWithdrawalTransactionIndex', {}, response);
    }

    return response;
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
    let response;

    try {
      response = await this.stateRepository.enqueueWithdrawalTransaction(
        index,
        transactionBytes,
      );
    } finally {
      this.log('enqueueWithdrawalTransaction', { index, transactionBytes }, response);
    }
  }

  /**
   * Calculates storage fee to epochs distribution amount and leftovers
   *
   * @param {number} storageFee
   * @param {number} startEpochIndex
   * @returns {Promise<[number, number]>}
   */
  async calculateStorageFeeDistributionAmountAndLeftovers(storageFee, startEpochIndex) {
    let response;

    try {
      response = await this.stateRepository.calculateStorageFeeDistributionAmountAndLeftovers(
        storageFee,
        startEpochIndex,
      );
    } finally {
      this.log('calculateStorageFeeDistributionAmountAndLeftovers', { storageFee, startEpochIndex }, response);
    }
  }
}

module.exports = LoggedStateRepositoryDecorator;
