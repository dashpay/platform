const Identity = require('@dashevo/dpp/lib/identity/Identity');

const RepositoryResult = require('../storage/RepositoryResult');

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
   * @return {Promise<RepositoryResult<void>>}
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

    return new RepositoryResult(
      undefined,
      result.getOperations(),
    );
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<null|Identity>>}
   */
  async fetch(id, useTransaction = false) {
    const encodedIdentityResult = await this.storage.get(
      IdentityStoreRepository.TREE_PATH,
      id.toBuffer(),
      { useTransaction },
    );

    if (!encodedIdentityResult.getResult()) {
      return new RepositoryResult(
        null,
        encodedIdentityResult.getOperations(),
      );
    }

    const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
      encodedIdentityResult.getResult(),
    );

    rawIdentity.protocolVersion = protocolVersion;

    return new RepositoryResult(
      new Identity(rawIdentity),
      encodedIdentityResult.getOperations(),
    );
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<RepositoryResult<void>>}
   */
  async createTree(options = {}) {
    const result = await this.storage.createTree(
      [],
      IdentityStoreRepository.TREE_PATH[0],
      options,
    );

    return new RepositoryResult(
      undefined,
      result.getOperations(),
    );
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

module.exports = IdentityStoreRepository;
