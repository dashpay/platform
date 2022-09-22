const { TYPES } = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const SignatureVerificationOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/SignatureVerificationOperation');

/**
 * @implements StateRepository
 */
class DriveStateRepository {
  #options = {};

  /**
   * @param {IdentityStoreRepository} identityRepository
   * @param {PublicKeyToIdentitiesStoreRepository} publicKeyToToIdentitiesRepository
   * @param {DataContractStoreRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {DocumentRepository} documentRepository
   * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {Object} [options]
   * @param {Object} [options.useTransaction=false]
   */
  constructor(
    identityRepository,
    publicKeyToToIdentitiesRepository,
    dataContractRepository,
    fetchDocuments,
    documentRepository,
    spentAssetLockTransactionsRepository,
    coreRpcClient,
    blockExecutionContext,
    simplifiedMasternodeList,
    options = {},
  ) {
    this.identityRepository = identityRepository;
    this.publicKeyToIdentitiesRepository = publicKeyToToIdentitiesRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.documentRepository = documentRepository;
    this.spentAssetLockTransactionsRepository = spentAssetLockTransactionsRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionContext = blockExecutionContext;
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.#options = options;
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
    const result = await this.identityRepository.fetch(
      id,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }

    return result.getValue();
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
    const result = await this.identityRepository.create(
      identity,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const result = await this.identityRepository.update(
      identity,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    for (const publicKeyHash of publicKeyHashes) {
      const result = await this.publicKeyToIdentitiesRepository.store(
        publicKeyHash,
        identityId,
        this.#createRepositoryOptions(executionContext),
      );

      if (executionContext) {
        executionContext.addOperation(...result.getOperations());
      }
    }
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
    const result = await this.spentAssetLockTransactionsRepository.store(
      outPointBuffer,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const result = await this.spentAssetLockTransactionsRepository.fetch(
      outPointBuffer,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }

    return !result.isNull();
  }

  /**
   * Fetch identity ids by related public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Array<Identifier[]>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes, executionContext = undefined) {
    // Keep await here.
    // noinspection UnnecessaryLocalVariableJS
    const results = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        this.publicKeyToIdentitiesRepository.fetch(
          publicKeyHash,
          this.#createRepositoryOptions(executionContext),
        )
      )),
    );

    return results.map((result) => {
      if (executionContext) {
        executionContext.addOperation(...result.getOperations());
      }

      return result.getValue();
    });
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
    const result = await this.dataContractRepository.fetch(
      id,
      {
        dryRun: executionContext ? executionContext.isDryRun() : false,
        // Transaction is not using since Data Contract
        // should be always committed to use
        useTransaction: false,
      },
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }

    return result.getValue();
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
    const result = await this.dataContractRepository.store(
      dataContract,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const result = await this.fetchDocumentsFunction(
      contractId,
      type,
      {
        ...options,
        ...this.#createRepositoryOptions(executionContext),
      },
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }

    return result.getValue();
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
    const result = await this.documentRepository.create(
      document,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const result = await this.documentRepository.update(
      document,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const result = await this.documentRepository.delete(
      dataContract,
      type,
      id,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
  }

  /**
   * Fetch Core transaction by ID
   *
   * @param {string} id - Transaction ID hex
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<Object|null>}
   */
  async fetchTransaction(id, executionContext = undefined) {
    if (executionContext && executionContext.isDryRun()) {
      executionContext.addOperation(
        new ReadOperation(512),
      );

      return {
        data: Buffer.alloc(0),
        height: 1,
      };
    }

    try {
      const { result: transaction } = await this.coreRpcClient.getRawTransaction(id, 1);

      const data = Buffer.from(transaction.hex, 'hex');

      if (executionContext) {
        executionContext.addOperation(
          new ReadOperation(data.length),
        );
      }

      return {
        data,
        height: transaction.height,
      };
    } catch (e) {
      // Invalid address or key error
      if (e.code === -5) {
        return null;
      }

      throw e;
    }
  }

  /**
   * Fetch the latest platform block height
   *
   * @return {Promise<Long>}
   */
  async fetchLatestPlatformBlockHeight() {
    return this.blockExecutionContext.getHeight();
  }

  /**
   * Fetch the latest platform block time
   *
   * @return {Promise<protobuf.Timestamp>}
   */
  async fetchLatestPlatformBlockTime() {
    return this.blockExecutionContext.getTime();
  }

  /**
   * Fetch the latest platform core chainlocked height
   *
   * @return {Promise<number>}
   */
  async fetchLatestPlatformCoreChainLockedHeight() {
    return this.blockExecutionContext.getCoreChainLockedHeight();
  }

  /**
   * Verify instant lock
   *
   * @param {InstantLock} instantLock
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<boolean>}
   */
  // eslint-disable-next-line no-unused-vars
  async verifyInstantLock(instantLock, executionContext = undefined) {
    const coreChainLockedHeight = this.blockExecutionContext.getCoreChainLockedHeight();

    if (coreChainLockedHeight === null) {
      return false;
    }

    if (executionContext) {
      executionContext.addOperation(
        new SignatureVerificationOperation(TYPES.ECDSA_SECP256K1),
      );

      if (executionContext.isDryRun()) {
        return true;
      }
    }

    try {
      const { result: isVerified } = await this.coreRpcClient.verifyIsLock(
        instantLock.getRequestId().toString('hex'),
        instantLock.txid,
        instantLock.signature,
        coreChainLockedHeight,
      );

      return isVerified;
    } catch (e) {
      // Invalid address or key error or
      // Invalid, missing or duplicate parameter
      // Parse error
      if ([-8, -5, -32700].includes(e.code)) {
        return false;
      }

      throw e;
    }
  }

  /**
   * Fetch Simplified Masternode List Store
   *
   * @return {Promise<SimplifiedMNListStore>}
   */
  async fetchSMLStore() {
    return this.simplifiedMasternodeList.getStore();
  }

  /**
   * @private
   * @param {StateTransitionExecutionContext} [executionContext]
   * @return {{dryRun: boolean, useTransaction: boolean}}
   */
  #createRepositoryOptions(executionContext) {
    return {
      useTransaction: this.#options.useTransaction || false,
      dryRun: executionContext ? executionContext.isDryRun() : false,
    };
  }
}

module.exports = DriveStateRepository;
