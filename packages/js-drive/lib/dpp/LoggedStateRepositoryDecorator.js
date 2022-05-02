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
   * Store identity
   *
   * @param {Identity} identity
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async storeIdentity(identity, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.storeIdentity(identity, executionContext);
    } finally {
      this.log(
        'storeIdentity',
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
   * Fetch identity ids mapped by related public keys
   * using public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Array<Identifier[]>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.fetchIdentityIdsByPublicKeyHashes(
        publicKeyHashes,
        executionContext,
      );
    } finally {
      this.log(
        'fetchIdentityIdsByPublicKeyHashes',
        {
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
  async storeDataContract(dataContract, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.storeDataContract(dataContract, executionContext);
    } finally {
      this.log('storeDataContract', { dataContract }, response);
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
   * Store document
   *
   * @param {Document} document
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async storeDocument(document, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.storeDocument(document, executionContext);
    } finally {
      this.log(
        'storeDocument',
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
   * @param {Identifier} contractId
   * @param {string} type
   * @param {Identifier} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<void>}
   */
  async removeDocument(contractId, type, id, executionContext = undefined) {
    let response;

    try {
      response = await this.stateRepository.removeDocument(contractId, type, id, executionContext);
    } finally {
      this.log(
        'removeDocument',
        {
          contractId,
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
   * Fetch latest platform block header
   *
   * @return {Promise<IHeader>}
   */
  async fetchLatestPlatformBlockHeader() {
    let response;

    try {
      response = await this.stateRepository.fetchLatestPlatformBlockHeader();
    } finally {
      this.log('fetchLatestPlatformBlockHeader', { }, response);
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
}

module.exports = LoggedStateRepositoryDecorator;
