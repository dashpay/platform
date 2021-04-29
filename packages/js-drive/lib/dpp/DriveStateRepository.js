class DriveStateRepository {
  /**
   * @param {IdentityStoreRepository} identityRepository
   * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
   * @param {DataContractStoreRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {DocumentIndexedStoreRepository} documentRepository
   * @param {SpentAssetLockTransactionsRepository} spentAssetLockTransactionsRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionContext} blockExecutionContext
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {FeatureFlagTypes} featureFlagTypes
   * @param {getLatestFeatureFlag} getLatestFeatureFlag
   * @param {BlockExecutionStoreTransactions} [blockExecutionStoreTransactions]
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
    featureFlagTypes,
    getLatestFeatureFlag,
    blockExecutionStoreTransactions = undefined,
  ) {
    this.identityRepository = identityRepository;
    this.publicKeyToIdentityIdRepository = publicKeyToIdentityIdRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.documentRepository = documentRepository;
    this.spentAssetLockTransactionsRepository = spentAssetLockTransactionsRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionStoreTransactions = blockExecutionStoreTransactions;
    this.blockExecutionContext = blockExecutionContext;
    this.simplifiedMasternodeList = simplifiedMasternodeList;
    this.featureFlagTypes = featureFlagTypes;
    this.getLatestFeatureFlag = getLatestFeatureFlag;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    const transaction = this.getDBTransaction('identities');

    return this.identityRepository.fetch(id, transaction);
  }

  /**
   * Store identity
   *
   * @param {Identity} identity
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    const transaction = this.getDBTransaction('identities');

    await this.identityRepository.store(identity, transaction);
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
    const transaction = this.getDBTransaction('publicKeyToIdentityId');

    await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => this.publicKeyToIdentityIdRepository
        .store(
          publicKeyHash, identityId, transaction,
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
    const transaction = this.getDBTransaction('assetLockTransactions');

    this.spentAssetLockTransactionsRepository.store(
      outPointBuffer,
      transaction,
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
    const transaction = this.getDBTransaction('assetLockTransactions');

    const result = this.spentAssetLockTransactionsRepository.fetch(
      outPointBuffer,
      transaction,
    );

    return result !== null;
  }

  /**
   * Fetch identity ids by related public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   *
   * @returns {Promise<Array<Identifier|null>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes) {
    const transaction = this.getDBTransaction('publicKeyToIdentityId');

    // Keep await here.
    // noinspection UnnecessaryLocalVariableJS
    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        this.publicKeyToIdentityIdRepository.fetch(
          publicKeyHash, transaction,
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
    const transaction = this.getDBTransaction('dataContracts');

    await this.dataContractRepository.store(dataContract, transaction);
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
    const transaction = this.getDBTransaction('documents');

    return this.fetchDocumentsFunction(contractId, type, options, transaction);
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @returns {Promise<void>}
   */
  async storeDocument(document) {
    const transaction = this.getDBTransaction('documents');

    await this.documentRepository.store(document, transaction);
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
    const transaction = this.getDBTransaction('documents');

    await this.documentRepository.delete(contractId, type, id, transaction);
  }

  /**
   * Fetch Core transaction by ID
   *
   * @param {string} id
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

    if (!header) {
      return false;
    }

    const {
      height: blockHeight,
      coreChainLockedHeight,
    } = header;

    const verifyLLMQSignaturesWithCoreFeatureFlag = await this.getLatestFeatureFlag(
      this.featureFlagTypes.VERIFY_LLMQ_SIGS_WITH_CORE,
      blockHeight,
      this.getDBTransaction('documents'),
    );

    if (!verifyLLMQSignaturesWithCoreFeatureFlag || !verifyLLMQSignaturesWithCoreFeatureFlag.get('enabled')) {
      // Here dashcore lib is used to verify signatures,
      // but this approach doesn’t handle signatures created by old quorums
      // that’a why a Core RPC method is used otherwise
      return instantLock.verify(this.simplifiedMasternodeList.getStore());
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
   * @private
   * @param {string} name
   * @return {MerkDbTransaction|DocumentsIndexedTransaction}
   */
  getDBTransaction(name) {
    let transaction;

    if (this.blockExecutionStoreTransactions) {
      transaction = this.blockExecutionStoreTransactions.getTransaction(name);
    }

    return transaction;
  }
}

module.exports = DriveStateRepository;
