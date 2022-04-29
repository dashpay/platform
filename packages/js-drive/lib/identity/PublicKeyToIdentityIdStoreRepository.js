const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

const RepositoryResult = require('../storage/RepositoryResult');

class PublicKeyToIdentityIdStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store public key to identity ids map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<RepositoryResult<void>>}
   */
  async store(publicKeyHash, identityId, useTransaction = false) {
    const existingIdsResult = await this.fetchBuffer(publicKeyHash, useTransaction);

    let identityIds = [];
    if (existingIdsResult.getResult()) {
      identityIds = cbor.decode(existingIdsResult.getResult());
    }

    let operations = existingIdsResult.getOperations();

    if (identityIds.find((id) => id.equals(identityId)) === undefined) {
      identityIds.push(identityId.toBuffer());

      const data = cbor.encode(identityIds);

      const result = await this.storage.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        data,
        { useTransaction },
      );

      operations = operations.concat(result.getOperations());
    }

    return new RepositoryResult(undefined, operations);
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<RepositoryResult<Buffer|null>>}
   */
  async fetchBuffer(publicKeyHash, useTransaction = false) {
    const result = await this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      { useTransaction },
    );

    return new RepositoryResult(
      result.getResult(),
      result.getOperations(),
    );
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<RepositoryResult<Identifier[]>>}
   */
  async fetch(publicKeyHash, useTransaction = false) {
    const existingIdsResult = await this.fetchBuffer(
      publicKeyHash, useTransaction,
    );

    if (!existingIdsResult.getResult()) {
      return new RepositoryResult(
        [],
        existingIdsResult.getOperations(),
      );
    }

    const identityIds = cbor.decode(existingIdsResult.getResult());

    return new RepositoryResult(
      identityIds.map((id) => new Identifier(id)),
      existingIdsResult.getOperations(),
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
      PublicKeyToIdentityIdStoreRepository.TREE_PATH[0],
      options,
    );

    return new RepositoryResult(
      undefined,
      result.getOperations(),
    );
  }
}

PublicKeyToIdentityIdStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = PublicKeyToIdentityIdStoreRepository;
