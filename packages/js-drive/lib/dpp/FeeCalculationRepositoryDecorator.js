const Delete = require("../fees/Delete");
const Write = require("../fees/Write");

class FeeCalculationRepositoryDecorator {
  /**
   * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
   */
  constructor(
    stateRepository,
  ) {
    this.stateRepository = stateRepository;
    this.operations = [];
  }

  /**
   * Reset operations counter
   */
  reset() {
    this.operations = [];
  }

  /**
   * Get current operations
   * 
   * @returns {Operation[]}
   */
  getOperations() {
    return this.operations;
  }

  /**
   * Fetch Identity by ID
   *
   * @param {Identifier} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    return this.stateRepository.fetchIdentity(id);;
  }

  /**
   * Store identity
   *
   * @param {Identity} identity
   * @returns {Promise<void>}
   */
  async storeIdentity(identity) {
    this.operations.push(
      new Write(
        identity.getId().length, 
        identity.toBuffer().length,
      ),
    );

    return this.stateRepository.storeIdentity(identity);;
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
    return this.stateRepository
      .storeIdentityPublicKeyHashes(identityId, publicKeyHashes);
  }

  /**
   * Fetch identity ids mapped by related public keys
   * using public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   *
   * @returns {Promise<Array<Identifier[]>>}
   */
  async fetchIdentityIdsByPublicKeyHashes(publicKeyHashes) {
    return this.stateRepository.fetchIdentityIdsByPublicKeyHashes(publicKeyHashes);
  }

  /**
   * Store spent asset lock transaction
   *
   * @param {Buffer} outPointBuffer
   *
   * @return {Promise<void>}
   */
  async markAssetLockTransactionOutPointAsUsed(outPointBuffer) {
    return this.stateRepository.markAssetLockTransactionOutPointAsUsed(outPointBuffer);
  }

  /**
   * Check if spent asset lock transaction is stored
   *
   * @param {Buffer} outPointBuffer
   *
   * @return {Promise<boolean>}
   */
  async isAssetLockTransactionOutPointAlreadyUsed(outPointBuffer) {
    return this.stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
      outPointBuffer,
    );
  }

  /**
   * Fetch Data Contract by ID
   *
   * @param {Identifier} id
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id) {
    return this.stateRepository.fetchDataContract(id);
  }

  /**
   * Store Data Contract
   *
   * @param {DataContract} dataContract
   * @returns {Promise<void>}
   */
  async storeDataContract(dataContract) {
    this.operations.push(
      new Write(
        dataContract.getId().length, 
        dataContract.toBuffer().length,
      ),
    );

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
    this.operations.push(
      new Write(
        document.getId().length, 
        document.toBuffer().length,
      ),
    );

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
    const dataContract = await this.fetchDataContract(contractId);

    // TODO: maybe check of contract exists

    this.operations.push(
      new Delete(
        contractId.length, 
        dataContract.toBuffer().length,
      ),
    );

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

  /**
   * Verify instant lock
   *
   * @param {InstantLock} instantLock
   *
   * @return {Promise<boolean>}
   */
  async verifyInstantLock(instantLock) {
    return this.stateRepository.verifyInstantLock(instantLock);
  }
}

module.exports = FeeCalculationRepositoryDecorator;