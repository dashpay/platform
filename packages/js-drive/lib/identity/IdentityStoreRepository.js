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
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async create(identity, options = {}) {
    if (options.dryRun) {
      return new StorageResult(undefined, []);
    }

    const key = identity.getId().toBuffer();
    const value = identity.toBuffer();

    const treeResult = await this.storage.createTree(
      IdentityStoreRepository.TREE_PATH,
      key,
      options,
    );

    const identityResult = await this.storage.put(
      IdentityStoreRepository.TREE_PATH.concat([key]),
      IdentityStoreRepository.IDENTITY_KEY,
      value,
      options,
    );

    return new StorageResult(
      undefined,
      treeResult.getOperations().concat(identityResult.getOperations()),
    );
  }

  /**
   * Store identity into database
   *
   * @param {Identity} identity
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async update(identity, options = {}) {
    if (options.dryRun) {
      return new StorageResult(undefined, []);
    }

    const key = identity.getId().toBuffer();
    const value = identity.toBuffer();

    const result = await this.storage.put(
      IdentityStoreRepository.TREE_PATH.concat([key]),
      IdentityStoreRepository.IDENTITY_KEY,
      value,
      options,
    );

    result.setValue(undefined);

    return result;
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<null|Identity>>}
   */
  async fetch(id, options = { }) {
    if (options.dryRun) {
      return new StorageResult(null, []);
    }

    let encodedIdentityResult;
    try {
      encodedIdentityResult = await this.storage.get(
        IdentityStoreRepository.TREE_PATH.concat([id.toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        options,
      );
    } catch (e) {
      if (!e.message.startsWith('path parent layer not found')) {
        throw e;
      }

      encodedIdentityResult = new StorageResult(
        null,
        [],
      );
    }

    if (encodedIdentityResult.isNull()) {
      return encodedIdentityResult;
    }

    const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
      encodedIdentityResult.getValue(),
    );

    rawIdentity.protocolVersion = protocolVersion;

    return new StorageResult(
      new Identity(rawIdentity),
      encodedIdentityResult.getOperations(),
    );
  }

  /**
   * Prove identity by id
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   *
   * @return {Promise<StorageResult<Buffer|null>>}
   * */
  async prove(id, options) {
    return this.proveMany([id], options);
  }

  /**
   * Prove identity by ids
   *
   * @param {Identifier[]} ids
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   *
   * @return {Promise<StorageResult<Buffer>>}
   * */
  async proveMany(ids, options) {
    const items = ids.map((id) => ({
      type: 'key',
      key: id.toBuffer(),
    }));

    return this.storage.proveQuery({
      path: IdentityStoreRepository.TREE_PATH,
      query: {
        query: {
          items,
          subqueryKey: IdentityStoreRepository.IDENTITY_KEY,
        },
      },
    }, options);
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

IdentityStoreRepository.IDENTITY_KEY = Buffer.from([0]);

module.exports = IdentityStoreRepository;
