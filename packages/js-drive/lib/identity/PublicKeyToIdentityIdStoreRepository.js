const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

const StorageResult = require('../storage/StorageResult');

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
   * @return {Promise<StorageResult<void>>}
   */
  async store(publicKeyHash, identityId, useTransaction = false) {
    const existingIdsResult = await this.fetchBuffer(publicKeyHash, useTransaction);

    let identityIds = [];
    if (existingIdsResult.getValue()) {
      identityIds = cbor.decode(existingIdsResult.getValue());
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

    return new StorageResult(undefined, operations);
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<StorageResult<Buffer|null>>}
   */
  async fetchBuffer(publicKeyHash, useTransaction = false) {
    const result = await this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      { useTransaction },
    );

    return new StorageResult(
      result.getValue(),
      result.getOperations(),
    );
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<StorageResult<Identifier[]>>}
   */
  async fetch(publicKeyHash, useTransaction = false) {
    const existingIdsResult = await this.fetchBuffer(
      publicKeyHash, useTransaction,
    );

    if (existingIdsResult.isNull()) {
      return new StorageResult(
        [],
        existingIdsResult.getOperations(),
      );
    }

    const identityIds = cbor.decode(existingIdsResult.getValue());

    return new StorageResult(
      identityIds.map((id) => new Identifier(id)),
      existingIdsResult.getOperations(),
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
      PublicKeyToIdentityIdStoreRepository.TREE_PATH[0],
      options,
    );
  }
}

PublicKeyToIdentityIdStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = PublicKeyToIdentityIdStoreRepository;
