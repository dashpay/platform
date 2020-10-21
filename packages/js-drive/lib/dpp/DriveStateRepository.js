class DriveStateRepository {
  /**
   * @param {IdentityLevelDBRepository} identityRepository
   * @param {PublicKeyIdentityIdMapLevelDBRepository} publicKeyIdentityIdMapLevelDBRepository
   * @param {DataContractLevelDBRepository} dataContractRepository
   * @param {fetchDocuments} fetchDocuments
   * @param {createDocumentMongoDbRepository} createDocumentRepository
   * @param {RpcClient} coreRpcClient
   * @param {BlockExecutionState} blockExecutionState
   * @param {BlockExecutionDBTransactions} [blockExecutionDBTransactions]
   */
  constructor(
    identityRepository,
    publicKeyIdentityIdMapLevelDBRepository,
    dataContractRepository,
    fetchDocuments,
    createDocumentRepository,
    coreRpcClient,
    blockExecutionState,
    blockExecutionDBTransactions = undefined,
  ) {
    this.identityRepository = identityRepository;
    this.publicKeyIdentityIdMapLevelDBRepository = publicKeyIdentityIdMapLevelDBRepository;
    this.dataContractRepository = dataContractRepository;
    this.fetchDocumentsFunction = fetchDocuments;
    this.createDocumentRepository = createDocumentRepository;
    this.coreRpcClient = coreRpcClient;
    this.blockExecutionDBTransactions = blockExecutionDBTransactions;
    this.blockExecutionState = blockExecutionState;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    const transaction = this.getDBTransaction('identity');

    return this.identityRepository.fetch(id, transaction);
  }

  /**
   * Store identity
   *
   * @param {Identity} identity
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    const transaction = this.getDBTransaction('identity');

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
    const transaction = this.getDBTransaction('identity');

    await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => this.publicKeyIdentityIdMapLevelDBRepository
        .store(
          publicKeyHash, identityId, transaction,
        )),
    );
  }

  /**
   * Fetch identity ids by related public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   *
   * @returns {Promise<Array<Identifier|null>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes) {
    const transaction = this.getDBTransaction('identity');

    // Keep await here.
    // noinspection UnnecessaryLocalVariableJS
    const identityIds = await Promise.all(
      publicKeyHashes.map(async (publicKeyHash) => (
        this.publicKeyIdentityIdMapLevelDBRepository.fetch(
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
    const transaction = this.getDBTransaction('dataContract');

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
    const transaction = this.getDBTransaction('document');

    return this.fetchDocumentsFunction(contractId, type, options, transaction);
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @returns {Promise<void>}
   */
  async storeDocument(document) {
    const transaction = this.getDBTransaction('document');

    const repository = await this.createDocumentRepository(
      document.getDataContractId(),
      document.getType(),
    );

    await repository.store(document, transaction);
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
    const transaction = this.getDBTransaction('document');

    const repository = await this.createDocumentRepository(
      contractId,
      type,
    );

    await repository.delete(id, transaction);
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

      return transaction;
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
    return this.blockExecutionState.getHeader();
  }

  /**
   * @private
   * @param {string} name
   * @return {LevelDBTransaction|MongoDBTransaction}
   */
  getDBTransaction(name) {
    let transaction;

    if (this.blockExecutionDBTransactions) {
      transaction = this.blockExecutionDBTransactions.getTransaction(name);
    }

    return transaction;
  }
}

module.exports = DriveStateRepository;
