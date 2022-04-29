const Identity = require('@dashevo/dpp/lib/identity/Identity');

const StorageResult = require('../storage/StorageResult');

class IdentityStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(groveDBStore, decodeProtocolEntity) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {boolean} [useTransaction=false]
   * @return {Promise<StorageResult<void>>}
   */
  async store(identity, useTransaction = false) {
    const key = identity.getId().toBuffer();
    const value = identity.toBuffer();

    const result = await this.storage.put(
      IdentityStoreRepository.TREE_PATH,
      key,
      value,
      { useTransaction },
    );

    result.setValue(undefined);

    return result;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {boolean} [useTransaction=false]
   * @return {Promise<StorageResult<null|Identity>>}
   */
  async fetch(id, useTransaction = false) {
    const encodedIdentityResult = await this.storage.get(
      IdentityStoreRepository.TREE_PATH,
      id.toBuffer(),
      { useTransaction },
    );

    if (encodedIdentityResult.isNull()) {
      return encodedIdentityResult;
    }

    const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
      encodedIdentityResult.getResult(),
    );

    rawIdentity.protocolVersion = protocolVersion;

    return new StorageResult(
      new Identity(rawIdentity),
      encodedIdentityResult.getOperations(),
    );
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async createTree(options = {}) {
    return this.storage.createTree(
      [],
      IdentityStoreRepository.TREE_PATH[0],
      options,
    );
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

module.exports = IdentityStoreRepository;
