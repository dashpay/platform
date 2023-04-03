const { InstantLock } = require('@dashevo/dashcore-lib');
const BlockInfo = require('../blockExecution/BlockInfo');

/**
 * @implements StateRepository
 */
class DriveStateRepository {
  #options = {};

  /**
   * @param {IdentityStoreRepository} identityRepository
   * @param {IdentityBalanceStoreRepository} identityBalanceRepository
   * @param {IdentityPublicKeyStoreRepository} publicKeyToToIdentitiesRepository
   * @param {DataContractStoreRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {DocumentRepository} documentRepository
   * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {Drive} rsDrive
   * @param {WebAssembly.Instance} dppWasm
   * @param {Object} [options]
   * @param {Object} [options.useTransaction=false]
   */
  constructor(
    identityRepository,
    identityBalanceRepository,
    publicKeyToToIdentitiesRepository,
    dataContractRepository,
    fetchDocuments,
    documentRepository,
    spentAssetLockTransactionsRepository,
    coreRpcClient,
    blockExecutionContext,
    simplifiedMasternodeList,
    rsDrive,
    dppWasm,
    options = {},
  ) {
    this.identityRepository = identityRepository;
    this.identityBalanceRepository = identityBalanceRepository;
    this.identityPublicKeyRepository = publicKeyToToIdentitiesRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.documentRepository = documentRepository;
    this.spentAssetLockTransactionsRepository = spentAssetLockTransactionsRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionContext = blockExecutionContext;
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.rsDrive = rsDrive;
    this.dppWasm = dppWasm;
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
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentity() start', {
      id, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.fetch(
      id,
      {
        blockInfo,
        ...this.#createRepositoryOptions(executionContext),
      },
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = result.getValue();
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentity() finished', value);
    return value;
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
    // TODO: TEST - remove
    console.log('DriveStateRepository.createIdentity() start', {
      identity, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.create(
      identity,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    // TODO: TEST - remove
    console.log('DriveStateRepository.createIdentity() end');
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
    // TODO: TEST - remove
    console.log('DriveStateRepository.addKeysToIdentity() start', {
      identityId, keys, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityPublicKeyRepository.add(
      identityId,
      keys.map((key) => key.toObject()),
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    // TODO: TEST - remove
    console.log('DriveStateRepository.addKeysToIdentity() end');
  }

  /**
   * Fetch identity balance
   *
   * @param {Identifier} identityId
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<number|null>}
   */
  async fetchIdentityBalance(identityId, executionContext = undefined) {
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentityBalance() start', {
      identityId, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityBalanceRepository.fetch(
      identityId,
      {
        blockInfo,
        ...this.#createRepositoryOptions(executionContext),
      },
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = result.getValue();
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentityBalance() end', {
      value,
    });

    return value;
  }

  /**
   * Fetch identity balance with debt
   *
   * @param {Identifier} identityId
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<number|null>} - Balance can be negative in case of debt
   */
  async fetchIdentityBalanceWithDebt(identityId, executionContext = undefined) {
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentityBalanceWitDebt() start', {
      identityId, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityBalanceRepository.fetchWithDebt(
      identityId,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = result.getValue();
    // TODO: TEST - remove
    console.log('DriveStateRepository.fetchIdentityBalanceWithDebt() end', {
      identityId, executionContext,
    });
    return value;
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
    // TODO: TEST - remove
    console.log('DriveStateRepository.addToIdentityBalance() start', {
      identityId, amount, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityBalanceRepository.add(
      identityId,
      amount,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    // TODO: TEST - remove
    console.log('DriveStateRepository.addToIdentityBalance() end', {});
  }

  /**
   * Add to system credits
   *
   * @param {number} amount
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async addToSystemCredits(amount, executionContext = undefined) {
    // TODO: TEST - remove
    console.log('DriveStateRepository.addToSystemCredits() start', {
      amount, executionContext,
    });
    if (executionContext.isDryRun()) {
      return;
    }

    await this.rsDrive.addToSystemCredits(
      amount,
      this.#options.useTransaction || false,
    );

    console.log('DriveStateRepository.addToSystemCredits() end', {});
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
    console.log('DriveStateRepository.disableIdentityKeys() start', {
      identityId, keyIds, disableAt, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityPublicKeyRepository.disable(
      identityId,
      keyIds,
      disableAt,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.disableIdentityKeys() end');
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
    console.log('DriveStateRepository.updateIdentityRevision() start', {
      identityId, revision, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.identityRepository.updateRevision(
      identityId,
      revision,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.updateIdentityRevision() end', {
    });
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
    console.log('DriveStateRepository.markAssetLockTransactionOutPointAsUsed() start', {
      outPointBuffer, executionContext,
    });
    const result = await this.spentAssetLockTransactionsRepository.store(
      outPointBuffer,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.markAssetLockTransactionOutPointAsUsed() end', {});
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
    console.log('DriveStateRepository.isAssetLockTransactionOutPointAlreadyUsed() start', {
      outPointBuffer, executionContext,
    });
    const result = await this.spentAssetLockTransactionsRepository.fetch(
      outPointBuffer,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = !result.isNull();

    console.log('DriveStateRepository.isAssetLockTransactionOutPointAlreadyUsed() end', {
      value,
    });

    return value;
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
    console.log('DriveStateRepository.fetchDataContract() start', {
      id, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.fetch(
      id,
      {
        ...this.#createRepositoryOptions(executionContext),
        // This method doesn't implement dry run because we need a contract
        // to proceed dry run validation and collect further operations
        dryRun: false,
        blockInfo,
      },
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = result.getValue();

    console.log('DriveStateRepository.fetchDataContract() end', {
      value,
    });

    return value;
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
    console.log('DriveStateRepository.createDataContract() start', {
      dataContract, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.create(
      dataContract,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.createDataContract() end', {});
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
    console.log('DriveStateRepository.updateDataContract() start', {
      dataContract, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.dataContractRepository.update(
      dataContract,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.updateDataContract() end', {
    });
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
    console.log('DriveStateRepository.fetchDocuments() start', {
      contractId, type, options, executionContext,
    });
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    const value = result.getValue();
    console.log('DriveStateRepository.fetchDocuments() end', {
      value,
    });
    return value;
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
    console.log('DriveStateRepository.createDocument() start', {
      document, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.create(
      document,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.createDocument() end', {
    });
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
    console.log('DriveStateRepository.updateDocument() start', {
      document, executionContext,
    });
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.update(
      document,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.updateDocument() end', {

    });
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
    console.log('DriveStateRepository.removeDocument() start', {
      dataContract, type, id, executionContext,
    });

    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.documentRepository.delete(
      dataContract,
      type,
      id,
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    console.log('DriveStateRepository.removeDocument() end', {
    });
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
    console.log('DriveStateRepository.fetchTransaction() start', {
      id, executionContext,
    });

    if (executionContext && executionContext.isDryRun()) {
      executionContext.addOperation(
        // TODO: Revisit this value
        new this.dppWasm.ReadOperation(512),
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
          new this.dppWasm.ReadOperation(data.length),
        );
      }

      console.log('DriveStateRepository.fetchTransaction() end', {
        data, height: transaction.height,
      });

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
    console.log('DriveStateRepository.fetchLatestPlatformBlockHeight() start', {});

    const value = this.blockExecutionContext.getHeight();

    console.log('DriveStateRepository.fetchLatestPlatformBlockHeight() end', {
      value,
    });
    return value;
  }

  /**
   * Fetch the latest platform block time
   *
   * @return {Promise<number>}
   */
  async fetchLatestPlatformBlockTime() {
    console.log('DriveStateRepository.fetchLatestPlatformBlockTime() start', {});
    const timeMs = this.blockExecutionContext.getTimeMs();

    if (!timeMs) {
      throw new Error('Time is not set');
    }

    console.log('DriveStateRepository.fetchLatestPlatformBlockTime() end', {
      timeMs,
    });
    return timeMs;
  }

  /**
   * Fetch the latest platform core chainlocked height
   *
   * @return {Promise<number>}
   */
  async fetchLatestPlatformCoreChainLockedHeight() {
    console.log('DriveStateRepository.fetchLatestPlatformCoreChainLockedHeight() start', {});
    const value = this.blockExecutionContext.getCoreChainLockedHeight();
    console.log('DriveStateRepository.fetchLatestPlatformCoreChainLockedHeight() end', {
      value,
    });
    return value;
  }

  /**
   * Verify instant lock
   *
   * @param {Buffer} rawInstantLock
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<boolean>}
   */
  // eslint-disable-next-line no-unused-vars
  async verifyInstantLock(rawInstantLock, executionContext = undefined) {
    console.log('DriveStateRepository.verifyInstantLock() start', {
      rawInstantLock, executionContext,
    });
    const coreChainLockedHeight = this.blockExecutionContext.getCoreChainLockedHeight();

    if (coreChainLockedHeight === null) {
      return false;
    }

    if (executionContext) {
      executionContext.addOperation(
        new this.dppWasm.SignatureVerificationOperation(this.dppWasm.KeyType.ECDSA_SECP256K1),
      );

      if (executionContext.isDryRun()) {
        return true;
      }
    }

    try {
      // TODO: Identifier/buffer issue - problem with Buffer shim:
      //  Without Buffer.from will fail
      const instantLock = new InstantLock(Buffer.from(rawInstantLock));
      const { result: isVerified } = await this.coreRpcClient.verifyIsLock(
        instantLock.getRequestId().toString('hex'),
        instantLock.txid,
        instantLock.signature,
        coreChainLockedHeight,
      );

      console.log('DriveStateRepository.verifyInstantLock() end', {
        isVerified,
      });

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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    return this.rsDrive.fetchLatestWithdrawalTransactionIndex(
      blockInfo,
      this.#options.useTransaction,
      this.#options.dryRun,
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
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    // TODO: handle dry run via passing state transition execution context
    return this.rsDrive.enqueueWithdrawalTransaction(
      index,
      transactionBytes,
      blockInfo,
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
