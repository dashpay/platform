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
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    let response;

    try {
      response = await this.stateRepository.fetchIdentity(id);
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
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    let response;

    try {
      response = await this.stateRepository.storeIdentity(identity);
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
   *
   * @returns {Promise<void>}
   */
  async storeIdentityPublicKeyHashes(identityId, publicKeyHashes) {
    let response;

    try {
      response = await this.stateRepository
        .storeIdentityPublicKeyHashes(identityId, publicKeyHashes);
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
   *
   * @returns {Promise<Array<Identifier|null>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes) {
    let response;

    try {
      response = await this.stateRepository.fetchIdentityIdsByPublicKeyHashes(publicKeyHashes);
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
   *
   * @return {Promise<void>}
   */
  async markAssetLockTransactionOutPointAsUsed(outPointBuffer) {
    let response;

    try {
      response = await this.stateRepository.markAssetLockTransactionOutPointAsUsed(outPointBuffer);
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
   *
   * @return {Promise<boolean>}
   */
  async isAssetLockTransactionOutPointAlreadyUsed(outPointBuffer) {
    let response;

    try {
      response = await this.stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
        outPointBuffer,
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
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id) {
    let response;

    try {
      response = await this.stateRepository.fetchDataContract(id);
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
   * @returns {Promise<void>}
   */
  async storeDataContract(dataContract) {
    let response;

    try {
      response = await this.stateRepository.storeDataContract(dataContract);
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
   * @returns {Promise<Document[]>}
   */
  async fetchDocuments(contractId, type, options = {}) {
    let response;

    try {
      response = await this.stateRepository.fetchDocuments(contractId, type, options);
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
   * @returns {Promise<void>}
   */
  async storeDocument(document) {
    let response;

    try {
      response = await this.stateRepository.storeDocument(document);
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
   * @returns {Promise<void>}
   */
  async removeDocument(contractId, type, id) {
    let response;

    try {
      response = await this.stateRepository.removeDocument(contractId, type, id);
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
   * @returns {Promise<Object|null>}
   */
  async fetchTransaction(id) {
    let response;

    try {
      response = await this.stateRepository.fetchTransaction(id);
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
   *
   * @return {Promise<boolean>}
   */
  async verifyInstantLock(instantLock) {
    let response;

    try {
      response = await this.stateRepository.verifyInstantLock(instantLock);
    } finally {
      this.log('verifyInstantLock', { instantLock }, response);
    }

    return response;
  }
}

module.exports = LoggedStateRepositoryDecorator;
