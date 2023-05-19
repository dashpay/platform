const { InstantLock } = require('@dashevo/dashcore-lib');
const { ReadOperation, SignatureVerificationOperation, KeyType } = require('@dashevo/wasm-dpp');

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
        ...this.#createRepositoryOptions(executionContext),
      },
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      keys.map((key) => key.toObject()),
      blockInfo,
      this.#createRepositoryOptions(executionContext),
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }
  }

  /**
   * Fetch identity balance
   *
   * @param {Identifier} identityId
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<number|null>}
   */
  async fetchIdentityBalance(identityId, executionContext = undefined) {
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

    return result.getValue();
  }

  /**
   * Fetch identity balance with debt
   *
   * @param {Identifier} identityId
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<number|null>} - Balance can be negative in case of debt
   */
  async fetchIdentityBalanceWithDebt(identityId, executionContext = undefined) {
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

    return result.getValue();
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
  }

  /**
   * Add to system credits
   *
   * @param {number} amount
   * @param {StateTransitionExecutionContext} [executionContext]
   * @returns {Promise<void>}
   */
  async addToSystemCredits(amount, executionContext = undefined) {
    if (executionContext.isDryRun()) {
      return;
    }

    await this.rsDrive.addToSystemCredits(
      amount,
      this.#options.useTransaction || false,
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
    }

    return result.getValue();
  }

  /**
   * Fetch Extended Documents by contract ID and type
   *
   * @param {Identifier} contractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @returns {Promise<ExtendedDocument[]>}
   */
  async fetchExtendedDocuments(contractId, type, options = {}, executionContext = undefined) {
    const blockInfo = BlockInfo.createFromBlockExecutionContext(this.blockExecutionContext);

    const result = await this.fetchDocumentsFunction(
      contractId,
      type,
      {
        blockInfo,
        ...options,
        ...this.#createRepositoryOptions(executionContext),
      },
      true,
    );

    if (executionContext) {
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
      for (const operation of result.getOperations()) {
        executionContext.addOperation(operation);
      }
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
   * @param {Buffer} rawInstantLock
   * @param {StateTransitionExecutionContext} [executionContext]
   *
   * @return {Promise<boolean>}
   */
  // eslint-disable-next-line no-unused-vars
  async verifyInstantLock(rawInstantLock, executionContext = undefined) {
    const coreChainLockedHeight = this.blockExecutionContext.getCoreChainLockedHeight();

    if (coreChainLockedHeight === null) {
      return false;
    }

    if (executionContext) {
      executionContext.addOperation(
        new SignatureVerificationOperation(KeyType.ECDSA_SECP256K1),
      );

      if (executionContext.isDryRun()) {
        return true;
      }
    }

    const smlStore = this.simplifiedMasternodeList.getStore();
    const offset = 8;
    const instantlockSML = smlStore.getSMLbyHeight(
      smlStore.getTipHeight() - offset + 1,
    );

    const instantLock = new InstantLock(Buffer.from(rawInstantLock));

    // below is a fix for DIP 24
    // see https://github.com/dashpay/dash/pull/5158
    const llmqType = instantlockSML.getInstantSendLLMQType();

    if (instantlockSML.isLLMQTypeRotated(llmqType)) {
      const quorum = instantLock.selectSignatoryRotatedQuorum(
        smlStore,
        instantLock.getRequestId(),
        offset,
      );

      // TODO: We should throw an error if quorum is not found?
      if (quorum) {
        const { result: quorumInfo } = await this.coreRpcClient.quorum('info', llmqType, quorum.quorumHash);

        if (quorumInfo.previousConsecutiveDKGFailures !== 0) {
          return false;
        }
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
   * Verifies that a given masternode id is in the current valid masternode list
   *
   * @param {Buffer} masternodeId
   * @returns {Promise<boolean>}
   */
  async isInTheValidMasterNodesList(masternodeId) {
    const smlStore = await this.fetchSMLStore();
    const validMasternodesList = smlStore.getCurrentSML().getValidMasternodesList();

    return !!validMasternodesList.find(
      (smlEntry) => Buffer.compare(masternodeId, Buffer.from(smlEntry.proRegTxHash, 'hex')) === 0,
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
