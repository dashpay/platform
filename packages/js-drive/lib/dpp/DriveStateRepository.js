const { calculateStorageFeeDistributionAmountAndLeftovers } = require('@dashevo/rs-drive');

const { TYPES } = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const SignatureVerificationOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/SignatureVerificationOperation');
const BlockInfo = require('../blockExecution/BlockInfo');

/**
 * @implements StateRepository
 */
class DriveStateRepository {
  #options = {};

  /**
   * @param {IdentityStoreRepository} identityRepository
   * @param {IdentityPublicKeyStoreRepository} publicKeyToToIdentitiesRepository
   * @param {DataContractStoreRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {DocumentRepository} documentRepository
   * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {RSDrive} rsDrive
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
    rsDrive,
    options = {},
  ) {
    this.identityRepository = identityRepository;
    this.identityPublicKeyRepository = publicKeyToToIdentitiesRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.documentRepository = documentRepository;
    this.spentAssetLockTransactionsRepository = spentAssetLockTransactionsRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionContext = blockExecutionContext;
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.rsDrive = rsDrive;
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.fetch(
      id,
      {
        blockInfo,
        ...this.#createRepositoryOptions(executionContext)
      },
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.create(
      identity,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityPublicKeyRepository.add(
      identityId,
      keys,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.addToBalance(
      identityId,
      amount,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityPublicKeyRepository.disable(
      identityId,
      keyIds,
      disableAt,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.updateRevision(
      identityId,
      revision,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
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
   * Fetch Data Contract by ID
   *
   * @param {Identifier} id
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id, executionContext = undefined) {
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.fetch(
      id,
      {
        blockInfo,
        // This method doesn't implement dry run because we need a contract
        // to proceed dry run validation and collect further operations
        dryRun: false,
        // Transaction is not using since Data Contract
        // should be always committed to use
        // TODO: We don't need this anymore
        useTransaction: false,
      },
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }

    return result.getValue();
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.create(
      dataContract,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      executionContext.addOperation(...result.getOperations());
    }
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.update(
      dataContract,
      blockInfo,
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.fetchDocumentsFunction(
      contractId,
      type,
      {
        blockInfo,
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.create(
      document,
      blockInfo,
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.update(
      document,
      blockInfo,
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.delete(
      dataContract,
      type,
      id,
      blockInfo,
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
        // TODO: Revisit this value
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
   * @return {Promise<number>}
   */
  async fetchLatestPlatformBlockTime() {
    const timeMs = this.blockExecutionContext.getTimeMs();

    if (!timeMs) {
      throw new Error('Time is not set');
    }

    return timeMs;
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
   * Fetch the latest withdrawal transaction index
   *
   * @returns {Promise<number>}
   */
  async fetchLatestWithdrawalTransactionIndex() {
    // TODO: handle dry run via passing state transition execution context
    return this.rsDrive.fetchLatestWithdrawalTransactionIndex(
      this.#options.useTransaction,
    );
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
    // TODO: handle dry run via passing state transition execution context
    return this.rsDrive.enqueueWithdrawalTransaction(
      index,
      transactionBytes,
      this.#options.useTransaction,
    );
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
