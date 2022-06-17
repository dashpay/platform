const Identity = require('@dashevo/dpp/lib/identity/Identity');

const getBiggestPossibleIdentity = require('@dashevo/dpp/lib/identity/getBiggestPossibleIdentity');

const StorageResult = require('../storage/StorageResult');

const MAX_IDENTITY_SIZE = getBiggestPossibleIdentity().toBuffer().length;

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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async create(identity, options = {}) {
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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async update(identity, options = {}) {
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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<null|Identity>>}
   */
  async fetch(id, options = { }) {
    const encodedIdentityResult = await this.storage.get(
      IdentityStoreRepository.TREE_PATH.concat([id.toBuffer()]),
      IdentityStoreRepository.IDENTITY_KEY,
      {
        ...options,
        predictedValueSize: MAX_IDENTITY_SIZE,
      },
    );

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
      IdentityStoreRepository.TREE_PATH[0],
      options,
    );
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

IdentityStoreRepository.IDENTITY_KEY = Buffer.from([0]);

module.exports = IdentityStoreRepository;
