class DriveStateRepository {
  #options = {};

  /**
   * @type {LRUCache}
   */
  #dataContractCache;

  /**
   * @param {IdentityStoreRepository} identityRepository
   * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
   * @param {DataContractStoreRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {DocumentRepository} documentRepository
   * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {LRUCache} dataContractCache
   * @param {Object} [options]
   * @param {Object} [options.useTransaction=false]
   */
  constructor(
    identityRepository,
    publicKeyToIdentityIdRepository,
    dataContractRepository,
    fetchDocuments,
    documentRepository,
    spentAssetLockTransactionsRepository,
    coreRpcClient,
    blockExecutionContext,
    simplifiedMasternodeList,
    dataContractCache,
    options,
  ) {
    this.identityRepository = identityRepository;
    this.publicKeyToIdentityIdRepository = publicKeyToIdentityIdRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.documentRepository = documentRepository;
    this.spentAssetLockTransactionsRepository = spentAssetLockTransactionsRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionContext = blockExecutionContext;
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.#dataContractCache = dataContractCache;
    this.#options = options;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    return this.identityRepository.fetch(id, this.#options.useTransaction || false);
  }

  /**
   * Store identity
   *
   * @param {Identity} identity
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    await this.identityRepository.store(identity, this.#options.useTransaction || false);
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
    await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => this.publicKeyToIdentityIdRepository
        .store(
          publicKeyHash, identityId, this.#options.useTransaction || false,
        )),
    );
  }

  /**
   * Store spent asset lock transaction
   *
   * @param {Buffer} outPointBuffer
   *
   * @return {Promise<void>}
   */
  async markAssetLockTransactionOutPointAsUsed(outPointBuffer) {
    this.spentAssetLockTransactionsRepository.store(
      outPointBuffer,
      this.#options.useTransaction || false,
    );
  }

  /**
   * Check if spent asset lock transaction is stored
   *
   * @param {Buffer} outPointBuffer
   *
   * @return {Promise<boolean>}
   */
  async isAssetLockTransactionOutPointAlreadyUsed(outPointBuffer) {
    const result = this.spentAssetLockTransactionsRepository.fetch(
      outPointBuffer,
      this.#options.useTransaction || false,
    );

    return result !== null;
  }

  /**
   * Fetch identity ids by related public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   *
   * @returns {Promise<Array<Identifier[]>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes) {
    // Keep await here.
    // noinspection UnnecessaryLocalVariableJS
    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        this.publicKeyToIdentityIdRepository.fetch(
          publicKeyHash, this.#options.useTransaction || false,
        )
      )),
    );

    return identityIds;
  }

  /**
   * Fetch Data Contract by ID
   *
   * @param {Identifier} id
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id) {
    // Data Contracts should be already committed before use
    // so we don't need transaction here

    return this.dataContractRepository.fetch(id);
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @returns {Promise<void>}
   */
  async storeDataContract(dataContract) {
    await this.dataContractRepository.store(dataContract, this.#options.useTransaction || false);
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
    return this.fetchDocumentsFunction(contractId, type, options, this.#options.useTransaction || false);
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @returns {Promise<void>}
   */
  async storeDocument(document) {
    await this.documentRepository.store(document, this.#options.useTransaction || false);
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
    const contractIdString = contractId.toString();

    // TODO: This is not very clean approach since we have already cached decorator
    //  to enable caching for the whole state repository
    let dataContract = this.#dataContractCache.get(contractIdString);

    if (!dataContract) {
      dataContract = await this.fetchDataContract(contractId);

      this.#dataContractCache.set(contractIdString, dataContract);
    }

    await this.documentRepository.delete(
      dataContract,
      type,
      id,
      this.#options.useTransaction || false,
    );
  }

  /**
   * Fetch Core transaction by ID
   *
   * @param {string} id - Transaction ID hex
   * @returns {Promise<Object|null>}
   */
  async fetchTransaction(id) {
    try {
      const { result: transaction } = await this.coreRpcClient.getRawTransaction(id, 1);

      return {
        data: Buffer.from(transaction.hex, 'hex'),
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
   * Fetch latest platform block header
   *
   * @return {Promise<IHeader>}
   */
  async fetchLatestPlatformBlockHeader() {
    return this.blockExecutionContext.getHeader();
  }

  /**
   * Verify instant lock
   *
   * @param {InstantLock} instantLock
   *
   * @return {Promise<boolean>}
   */
  async verifyInstantLock(instantLock) {
    const header = await this.blockExecutionContext.getHeader();

    const {
      coreChainLockedHeight,
    } = header;

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
}

module.exports = DriveStateRepository;
