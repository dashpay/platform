const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');

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
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(publicKeyHash, identityId, options = {}) {
    const existingIdsResult = await this.fetchBuffer(publicKeyHash, options);

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
        options,
      );

      operations = operations.concat(result.getOperations());
    }

    return new StorageResult(undefined, operations);
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<Buffer|null>>}
   */
  async fetchBuffer(publicKeyHash, options = {}) {
    const result = await this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      options,
    );

    // There is no way to predict, how many identities could have the same
    // keys so as the simple solution we won't count value size for this
    // operation at all

    return new StorageResult(
      result.getValue(),
      [new ReadOperation(0)],
    );
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<Identifier[]>>}
   */
  async fetch(publicKeyHash, options = {}) {
    const existingIdsResult = await this.fetchBuffer(
      publicKeyHash, options,
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
   * @param {boolean} [options.skipIfExists=false]
   * @param {boolean} [options.dryRun=false]
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
