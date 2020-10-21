class CachedStateRepositoryDecorator {
  /**
   * @param {DriveStateRepository} stateRepository
   * @param {LRUCache} dataContractCache
   */
  constructor(
    stateRepository,
    dataContractCache,
  ) {
    this.stateRepository = stateRepository;
    this.contractCache = dataContractCache;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    return this.stateRepository.fetchIdentity(id);
  }

  /**
   * Store identity
   *
   * @param {Identity} identity
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    return this.stateRepository.storeIdentity(identity);
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
    return this.stateRepository.storeIdentityPublicKeyHashes(identityId, publicKeyHashes);
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
    return this.stateRepository.fetchIdentityIdsByPublicKeyHashes(publicKeyHashes);
  }

  /**
   * Fetch Data Contract by ID
   *
   * @param {Identifier} id
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id) {
    const idString = id.toString();

    let dataContract = this.contractCache.get(idString);

    if (dataContract !== undefined) {
      return dataContract;
    }

    dataContract = await this.stateRepository.fetchDataContract(id);

    if (dataContract !== null) {
      this.contractCache.set(idString, dataContract);
    }

    return dataContract;
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @returns {Promise<void>}
   */
  async storeDataContract(dataContract) {
    return this.stateRepository.storeDataContract(dataContract);
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
    return this.stateRepository.fetchDocuments(contractId, type, options);
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @returns {Promise<void>}
   */
  async storeDocument(document) {
    return this.stateRepository.storeDocument(document);
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
    return this.stateRepository.removeDocument(contractId, type, id);
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @returns {Promise<Object|null>}
   */
  async fetchTransaction(id) {
    return this.stateRepository.fetchTransaction(id);
  }

  /**
   * Fetch latest platform block header
   *
   * @return {Promise<IHeader>}
   */
  async fetchLatestPlatformBlockHeader() {
    return this.stateRepository.fetchLatestPlatformBlockHeader();
  }
}

module.exports = CachedStateRepositoryDecorator;
